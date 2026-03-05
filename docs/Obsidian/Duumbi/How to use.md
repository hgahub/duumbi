---
tags:
  - tool/obsidian
  - doc/reference
status: final
created: 2026-02-05
updated: 2026-02-06
---
# How to use 
A Guide to Effective Obsidian Vault Usage

## Introduction

Welcome to this Obsidian Vault! The structure and methodology provided here will help you organize your thoughts, discover connections, and effectively build your knowledge base.  
  
This Vault is primarily built around the **[LYT (Linking Your Thinking)](https://www.linkingyourthinking.com/)** concept, where ideas, research, and developed materials are stored.

## Basic Philosophy

* Obsidian is your personal knowledge base, a place for thinking, brainstorming, and understanding deeper connections.
    * **Dots:** Atomic notes, a single specific idea or piece of information.
    * **Maps:** Higher-level notes that connect and contextualize the "Dots" (Table of Contents - TOC).
    * **Works:** More elaborate, longer documents that are created based on "Dots" and "Maps".
* **Linking Above All:** We ensure the connection between systems and notes with internal and external links.

## Library Structure in Detail

### `01 Atlas (Knowledge Base)/`

This is the heart of the vault; here you build and nurture your knowledge related to your startup idea.

* **`Dots (Atomic Ideas)/`**
    * **Contents:** Each individual thought, idea snippet, definition, research finding, description of an API endpoint, characteristic of a cloud service, etc., is in a separate note.
    * **Usage:** Strive for atomicity ("one note, one idea"). Name the files descriptively, e.g. `AI image recognition API idea.md` or `AWS Lambda cost efficiency.md`. Use relevant tags and Properties fields.
    * **Goal:** To create easily linkable, reusable knowledge elements.

* **`Maps (Overviews)/`**
    * **Contents:** Summary notes, Tables of Contents (MOCs) covering each major topic. These notes primarily contain links to relevant `DOTS` and other `MAPS` notes, as well as brief summaries and explanations.
    * **Usage:** Create maps for central themes such as: `[[Cloud Strategy Overview]]`, `[[AI Components Overview]]`, `[[Competitor Analysis Overview]]`. 
    * **Goal:** To ensure structure and navigation within the knowledge base, and to visualize connections (also in the graph view).

* **`Works (Developed Materials)/`**
    * **Content:** Larger, interconnected, more developed documents built from multiple "Dots" and "Maps." Please provide business plan outlines, more detailed specifications, research summaries, and presentation outlines.
    * **Usage:** These documents often represent the initial, more developed phase of a project before they might become a ClickUp project, or they take shape here as a result of a ClickUp project.
    * **Goal:** To present knowledge in a structured, coherent form, which already has a "product" character.

### `02 Resources (Assets and Tools)/`

Here we store the elements that help build and complement the knowledge base.

* **`Tag Pages (Tag Notes)/`**
    * **Content:** .You can create a dedicated note here for every major, frequently used `#tag` (e.g., `AI.md` has a `#ai` tag.
    * **Usage:** In this note, you can define the label, collect related key ideas, or even list all notes tagged with that label using a Dataview query.
    * **Goal:** To give labels deeper context and a central point of connection.

* **`Sources (References)/`**
    * **Content:** Notes from external sources (books, articles, websites, scientific publications). Every source can have its own note.
    * **Usage:** Record the most important information about the source (author, title, availability) and your own notes, summaries, and thoughts about it. Link these notes into the relevant `Dots` or `Maps` notes in Atlas.
    * **Goal:** To track the literature and information used, and to organize the research materials.

* **`Attachments (MediaFiles)/`**
    * **Content:** Images, PDFs, audio files, videos, and other files not in `.md` format.
    * **Usage:** In Obsidian's settings (Settings -> Files & Links -> Default location for new attachments), set this folder as the default for attachments (`02 Resources (Assets and Tools)/Attachments (MediaFiles)/`). So when you drag a file into a note, it automatically ends up here.
    * **Goal:** Organized storage of media files associated with notes.

### `03 Templates/`

* **Content:** Templates for commonly used note structures.
* **Usage:** Create templates for things like new ideas, meeting minutes, source processing, and technology assessments. Using a "Templates" core plugin (or the "Templater" community plugin), you can easily insert these into new notes.
* **Goal:** Ensure consistency and speed up note-taking.

### `04 Inbox (ToProcess)/`

* **Content:** Quick ideas, links, and thoughts that don't yet have a final home.
* **Usage:** When you need to quickly jot something down but don't have time to properly elaborate or categorize it, save it here. Regularly (e.g., daily or weekly) review this folder and process the items within: create `DOTS` notes from them, assign them to `MAPS`, or delete them if they are no longer relevant.
* **Goal:** Quick capture without interrupting the workflow, and maintaining a "clean desk" in the other folders.

## Using Key Obsidian Features

* **Internal Linking (`[[Wikilink]]`):** This is the heart and soul of Obsidian! Actively use the `[[` and `]]` characters to link your notes. Whenever a concept, idea, project, or person comes up that has (or should have) a separate note, link it in!
* **Címkék (Tags):** Use the `#` symbol for tagging. e.g. `#tool/obsidian`
* **Properties:** Structured metadata placed at the beginning of each note, between `---` markers.
    * **Suggested fields:**
        ```yaml
        ---
        aliases: [alternative names, synonyms]
        tags: [tag1, project/subproject, status/idea]
        created: YYYY-MM-DD HH:MM
        updated: YYYY-MM-DD HH:MM
        related_maps: ["[[Map Title 1]]", "[[Map Title 2]]"]
        source_key: [SOURCES_References/SourceNoteName] # If linked to a source
        status: idea | final | done | archived
        ---
        ```
    * You can later build complex queries on these using the Dataview plugin.

## General Naming Conventions

* **File names:** Make them descriptive and short (e.g., Meeting notes (e.g., `Meeting note.md`) can be useful for chronological order.
* **Consistency:** Be consistent in naming and using folders, files, and labels.

## Suggested Workflow

1.  **Quick Capture:** New ideas, links -> `00 Inbox (ToProcess)/`.
2.  **Processing:** Regularly empty your inbox:
    * From the ideas, create atomic `Dots` notes in the `01 Atlas (Knowledge Base)/Dots (Atomic Ideas)/ folder`.. 
    * Link the new `Dots` to existing `Maps`, or create new `Maps` if necessary.
3.  **Deepening and Connection:** Work thru the `DOTS` notes. Link to other notes. Use the graph view to discover relationships.
4.  **Development:** When multiple "Dots" and "Maps" come together to form a larger unit, create a `WORKS` document in the `01 Atlas (Knowledge Base)/Works (Developed Materials)`/ folder.
5.  **Resource Management:** HIf you use external sources, save them in the `02 Resources (Assets and Tools)/Sources (References)`/ folder and link them to the relevant notes.
6.  **Regular Review::** Occasionally review the notes in the `Maps (Overviews)` folder, update them, and make new connections.
