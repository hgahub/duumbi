//! Conservative binding checks before a generated service is launched.
//!
//! Resolve constant host/port arguments through local function calls and loads.
//! Graph IDs and module layouts are not prescribed; unknown binding arguments
//! fail the contract instead of risking a public listener.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use super::{ProcessFailure, ProcessStage, infrastructure, program_failure};

type Constants = BTreeMap<String, Value>;

pub(super) async fn validate(workspace: &Path, port: u16) -> Result<(), ProcessFailure> {
    let mut pending = vec![workspace.join(".duumbi/graph")];
    let mut modules = BTreeMap::new();
    while let Some(path) = pending.pop() {
        let mut entries = tokio::fs::read_dir(path)
            .await
            .map_err(|e| infrastructure(ProcessStage::Setup, e))?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| infrastructure(ProcessStage::Setup, e))?
        {
            let kind = entry
                .file_type()
                .await
                .map_err(|e| infrastructure(ProcessStage::Setup, e))?;
            if kind.is_symlink() {
                return Err(program_failure(
                    ProcessStage::Assertion,
                    "process graphs must not contain symlinks",
                ));
            }
            if kind.is_dir() {
                pending.push(entry.path());
            } else if entry.path().extension().is_some_and(|ext| ext == "jsonld") {
                let bytes = tokio::fs::read(entry.path())
                    .await
                    .map_err(|e| infrastructure(ProcessStage::Setup, e))?;
                let module: Value = serde_json::from_slice(&bytes)
                    .map_err(|e| infrastructure(ProcessStage::Setup, e))?;
                let name = module["duumbi:name"]
                    .as_str()
                    .ok_or_else(|| program_failure(ProcessStage::Assertion, "missing module name"))?
                    .to_string();
                modules.insert(name, module);
            }
        }
    }
    let mut checker = Checker {
        modules: &modules,
        port,
        bindings: 0,
        budget: 4096,
    };
    checker.function("main", "main", &[], 0)?;
    if checker.bindings == 0 {
        return Err(program_failure(
            ProcessStage::Assertion,
            "no verifiable loopback listener in the generated entry point",
        ));
    }
    Ok(())
}

struct Checker<'a> {
    modules: &'a BTreeMap<String, Value>,
    port: u16,
    bindings: usize,
    budget: usize,
}

impl Checker<'_> {
    fn function(
        &mut self,
        module_name: &str,
        name: &str,
        args: &[Value],
        depth: usize,
    ) -> Result<Value, ProcessFailure> {
        if depth > 32 || self.budget == 0 {
            return Err(program_failure(
                ProcessStage::Assertion,
                "cannot prove listener arguments within the binding-analysis bound",
            ));
        }
        self.budget -= 1;
        let Some(module) = self.modules.get(module_name) else {
            return Ok(Value::Null);
        };
        let Some(function) = module["duumbi:functions"]
            .as_array()
            .and_then(|functions| functions.iter().find(|f| f["duumbi:name"] == name))
        else {
            return Ok(Value::Null);
        };
        let params: Constants = function["duumbi:params"]
            .as_array()
            .into_iter()
            .flatten()
            .zip(args)
            .filter_map(|(p, v)| {
                p["duumbi:name"]
                    .as_str()
                    .map(|name| (name.to_string(), v.clone()))
            })
            .collect();
        let mut values = Constants::new();
        let mut returned = None;
        for op in function["duumbi:blocks"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|b| b["duumbi:ops"].as_array().into_iter().flatten())
        {
            let get = |reference: &Value| {
                reference["@id"]
                    .as_str()
                    .and_then(|id| values.get(id))
                    .cloned()
                    .unwrap_or(Value::Null)
            };
            let value = match op["@type"].as_str() {
                Some("duumbi:Const") => match &op["duumbi:value"] {
                    Value::String(value) if value.len() <= 256 => Value::String(value.clone()),
                    Value::Number(value) => Value::Number(value.clone()),
                    _ => Value::Null,
                },
                Some("duumbi:Load") => op["duumbi:variable"]
                    .as_str()
                    .and_then(|name| params.get(name))
                    .cloned()
                    .unwrap_or(Value::Null),
                Some("duumbi:StringConcat") => {
                    match (get(&op["duumbi:left"]), get(&op["duumbi:right"])) {
                        (Value::String(left), Value::String(right))
                            if left.len() + right.len() <= 256 =>
                        {
                            Value::String(left + &right)
                        }
                        _ => Value::Null,
                    }
                }
                Some("duumbi:ServerNew" | "duumbi:TcpListen") => {
                    self.binding(&get(&op["duumbi:operand"]), &get(&op["duumbi:left"]))?;
                    Value::Null
                }
                Some("duumbi:Call") => {
                    let target_module = op["duumbi:module"].as_str().unwrap_or(module_name);
                    let target = op["duumbi:function"].as_str().unwrap_or_default();
                    let args: Vec<Value> = op["duumbi:args"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(get)
                        .collect();
                    let imported = module["duumbi:imports"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .find(|i| i["duumbi:module"] == target_module);
                    let path = imported.and_then(|i| i["duumbi:path"].as_str());
                    if (path == Some("@duumbi/stdlib-server") && target == "server_new")
                        || (path == Some("@duumbi/stdlib-net") && target == "tcp_listen")
                    {
                        self.binding(
                            args.first().unwrap_or(&Value::Null),
                            args.get(1).unwrap_or(&Value::Null),
                        )?;
                        Value::Null
                    } else {
                        self.function(target_module, target, &args, depth + 1)?
                    }
                }
                Some("duumbi:Return") => {
                    let value = get(&op["duumbi:operand"]);
                    returned = Some(match returned {
                        None => value,
                        Some(previous) if previous == value => previous,
                        _ => Value::Null,
                    });
                    Value::Null
                }
                _ => Value::Null,
            };
            if let Some(id) = op["@id"].as_str() {
                values.insert(id.to_string(), value);
            }
        }
        Ok(returned.unwrap_or(Value::Null))
    }

    fn binding(&mut self, host: &Value, port: &Value) -> Result<(), ProcessFailure> {
        if host.as_str() != Some("127.0.0.1") || port.as_u64() != Some(u64::from(self.port)) {
            return Err(program_failure(
                ProcessStage::Assertion,
                "listener arguments must resolve to 127.0.0.1 and the supplied ephemeral port; computed or public addresses cannot be launched",
            ));
        }
        self.bindings += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn validates_forwarded_arguments_and_rejects_public_or_unknown_hosts() {
        for (host, valid) in [
            (json!("127.0.0.1"), true),
            (json!("0.0.0.0"), false),
            (Value::Null, false),
        ] {
            let tmp = tempfile::tempdir().expect("workspace");
            let dir = tmp.path().join(".duumbi/graph");
            std::fs::create_dir_all(&dir).expect("graph dir");
            let module = json!({"duumbi:name":"main", "duumbi:functions":[
                {"duumbi:name":"main", "duumbi:blocks":[{"duumbi:ops":[
                    {"@type":"duumbi:Const", "@id":"h", "duumbi:value":host},
                    {"@type":"duumbi:Const", "@id":"p", "duumbi:value":32123},
                    {"@type":"duumbi:Call", "@id":"call", "duumbi:function":"serve", "duumbi:args":[{"@id":"h"},{"@id":"p"}]}
                ]}]},
                {"duumbi:name":"serve", "duumbi:params":[{"duumbi:name":"host"},{"duumbi:name":"port"}], "duumbi:blocks":[{"duumbi:ops":[
                    {"@type":"duumbi:Load", "@id":"forwarded_h", "duumbi:variable":"host"},
                    {"@type":"duumbi:Load", "@id":"forwarded_p", "duumbi:variable":"port"},
                    {"@type":"duumbi:ServerNew", "@id":"server", "duumbi:operand":{"@id":"forwarded_h"}, "duumbi:left":{"@id":"forwarded_p"}}
                ]}]}
            ]});
            std::fs::write(dir.join("anything.jsonld"), module.to_string()).expect("graph");
            assert_eq!(validate(tmp.path(), 32123).await.is_ok(), valid);
            assert!(validate(tmp.path(), 32124).await.is_err());
        }
    }
}
