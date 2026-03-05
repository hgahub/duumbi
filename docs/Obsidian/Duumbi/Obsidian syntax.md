---
tags:
  - tool/obsidian
  - doc/reference
status: final
created: 2025-11-12T17:00:00
updated: 2025-11-12T17:00:00
---

# Obsidian syntax

### Formatting syntax

Learn how to apply basic formatting to your notes[^1], using [Markdown](https://daringfireball.net/projects/markdown/).
Text inside `backticks` on a line will be formatted like code. 😀
This is an inline math expression $\sum_{i=0}^n i^2 = \frac{(n^2+n)(2n+1)}{6}$.

|Style|Syntax|Example|Output|
|---|---|---|---|
|Bold|`** **` or `__ __`|`**Bold text**`|**Bold text**|
|Italic|`* *` or `_ _`|`*Italic text*`|_Italic text_|
|Strikethrough|`~~ ~~`|`~~Striked out text~~`|~~Striked out text~~|
|Highlight|`== ==`|`==Highlighted text==`|==Highlighted text==|
|Bold and nested italic|`** **` and `_ _`|`**Bold text and _nested italic_ text**`|**Bold text and _nested italic_ text**|
|Bold and italic|`*** ***` or `___ ___`|`***Bold and italic text***`|**_Bold and italic te_**|
#### Syntax

```python
if __name__ == "__main__":
    print("{title} \n".format(title=TITLE))

    template = Template("default")
    template.create()
```

#### Callouts

> [!tip] Callouts can have custom titles > Tip

> [!info] Callouts can have custom titles > Info

> [!note] Callouts can have custom titles > Note

> [!question] Callouts can have custom titles > Question

> [!example] Callouts can have custom titles > Example

> [!warning] Callouts can have custom titles > Warning

#### Quotes

> Human beings face ever more complex and urgent problems, and their effectiveness in dealing with these problems is a matter that is critical to the stability and continued progress of society.

#### Task list

- [x] Milk
- [?] Eggs
- [-] Eggs
- [ ] Butter
#### Tables

| C1  | C2  | C3  | C4  |
| --- | --- | --- | --- |
| 123 | 456 | 789 |     |
| 000 | 111 | 222 | 22  |

Make a note of something, [[create a link]], or try [the Importer](https://help.obsidian.md/Plugins/Importer)!

[^1]: This is the referenced text.