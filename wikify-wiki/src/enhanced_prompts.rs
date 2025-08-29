//! Enhanced prompt templates for direct markdown generation
//!
//! This module contains DeepWiki-inspired prompts that generate markdown content directly,
//! eliminating the need for intermediate format conversion and improving quality.

use crate::types::{ImportanceLevel, RepositoryInfo, WikiConfig, WikiPage};

/// Enhanced prompts for direct markdown generation
pub struct MarkdownPrompts;

impl MarkdownPrompts {
    /// Create a comprehensive prompt for direct markdown content generation
    /// This is inspired by DeepWiki's detailed formatting instructions
    pub fn create_direct_markdown_prompt(
        page: &WikiPage,
        relevant_files: &[String],
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
    ) -> String {
        let language_instruction = Self::get_language_instruction(&config.language);
        let importance_guidance = Self::get_importance_guidance(&page.importance);
        let file_list = Self::format_file_list(relevant_files, repo_info);

        format!(
            r#"You are an expert technical writer and software architect.
Your task is to generate a comprehensive and accurate technical wiki page in Markdown format about a specific feature, system, or module within a given software project.

{language_instruction}

You will be given:
1. The "[WIKI_PAGE_TOPIC]" for the page you need to create: "{}"
2. A list of "[RELEVANT_SOURCE_FILES]" from the project that you MUST use as the sole basis for the content.

CRITICAL STARTING INSTRUCTION:
The very first thing on the page MUST be a `<details>` block listing ALL the `[RELEVANT_SOURCE_FILES]` you used to generate the content.
Format it exactly like this:

<details>
<summary>📁 Source Files</summary>

The following files were used as context for generating this wiki page:

{}

</details>

Immediately after the `<details>` block, the main title of the page should be a H1 Markdown heading: `# {}`

Based ONLY on the content of the `[RELEVANT_SOURCE_FILES]`:

## 📋 Content Structure Requirements

### 1. **Introduction Section**
Start with a concise introduction (1-2 paragraphs) explaining:
- The purpose and scope of "{}"
- High-level overview within the project context
- Key relationships to other components (if evident from source files)

### 2. **Detailed Technical Sections**
Break down "{}" into logical sections using H2 (`##`) and H3 (`###`) Markdown headings:
- Explain architecture, components, data flow, or logic
- Identify key functions, classes, data structures, API endpoints
- Document configuration elements and their purposes

### 3. **Mermaid Diagrams (CRITICAL FORMATTING)**
EXTENSIVELY use Mermaid diagrams to visually represent:
- System architecture and component relationships
- Data flow and process sequences
- Class hierarchies and dependencies

**STRICT Mermaid Requirements:**
- ALWAYS use `graph TD` (top-down) directive for flow diagrams
- NEVER use `graph LR` (left-right) orientation
- Maximum node width should be 3-4 words
- For sequence diagrams:
  ```
  sequenceDiagram
      participant A as ComponentA
      participant B as ComponentB
      A->>B: Request
      B-->>A: Response
  ```
- Use descriptive but concise names
- Include activation boxes with +/- notation where relevant

### 4. **Structured Information Tables**
Use Markdown tables to organize:
- Key features/components and descriptions
- API endpoints, parameters, types, descriptions
- Configuration options, types, default values
- Data model fields, types, constraints

Example format:
| Component | Description | Status |
|-----------|-------------|--------|
| Feature A | Core functionality | ✅ Active |
| Feature B | Optional module | 🚧 Development |

### 5. **Code Examples (When Relevant)**
Include SHORT, relevant code snippets from the source files:
- Use proper language identifiers in code blocks
- Focus on key implementation patterns
- Highlight important configuration examples

### 6. **Source Citations (MANDATORY)**
For EVERY significant piece of information, you MUST cite the specific source file(s):
- Use format: `Sources: [filename.ext:line_range]()`
- Example: `Sources: [main.rs:45-67](), [config.rs:12]()`
- Cite at least 3-5 different source files throughout the page

### 7. **Navigation and Links**
- Link to related wiki pages using format: `[Page Name](./page-name.md)`
- Include "See Also" section if relevant
- Add navigation hints for next steps

## 🎯 Quality Standards

{}

### Technical Accuracy
- Base ALL information on provided source files
- Do not infer or invent details not present in the code
- If crucial information is missing, explicitly state its absence

### Clarity and Structure
- Use clear, professional technical language
- Maintain logical information flow
- Include practical examples where possible

### Completeness
- Cover all major aspects evident in the source files
- Ensure comprehensive documentation of the topic
- Balance detail with readability

## 📊 Expected Output Format

```markdown
<details>
<summary>📁 Source Files</summary>
[File list here]
</details>

# [Page Title]

## 📋 Overview
[Introduction and context]

## 🏗️ Architecture
[System design with Mermaid diagrams]

## 🔧 Key Components
[Detailed component breakdown]

## 📊 Configuration
[Settings and options in tables]

## 💻 Usage Examples
[Code snippets and examples]

## 🔗 Related Pages
[Links to other wiki pages]

## 📚 References
[Source citations]
```

Generate comprehensive, accurate, and well-structured markdown content that serves as definitive documentation for "{}".
"#,
            page.title,
            file_list,
            page.title,
            page.title,
            page.title,
            importance_guidance,
            page.title
        )
    }

    /// Create enhanced structure generation prompt with XML output
    pub fn create_enhanced_structure_prompt(
        repo_info: &RepositoryInfo,
        config: &WikiConfig,
        file_tree: &str,
    ) -> String {
        let language_instruction = Self::get_language_instruction(&config.language);

        format!(
            r#"You are an expert technical documentation architect specializing in creating comprehensive wiki structures for software projects.

{language_instruction}

## 📊 Repository Analysis

**Project Information:**
- Name: {}
- Description: {}
- Languages: {}
- Total Files: {}
- Has API: {}

**File Structure:**
```
{}
```

## 🎯 Task: Generate Comprehensive Wiki Structure

Create a well-organized wiki structure that helps developers understand and work with this codebase effectively.

## 📋 Output Requirements

Generate your response in the following XML format:

```xml
<wiki_structure>
  <metadata>
    <title>[Project name - clear and descriptive]</title>
    <description>[2-3 sentence project description]</description>
    <target_audience>developers, contributors, users</target_audience>
    <complexity_level>beginner|intermediate|advanced</complexity_level>
  </metadata>
  
  <sections>
    <section id="getting-started" order="1">
      <title>🚀 Getting Started</title>
      <description>Essential information for new users and developers</description>
      <pages>
        <page_ref>overview</page_ref>
        <page_ref>installation</page_ref>
        <page_ref>quick-start</page_ref>
      </pages>
    </section>
    
    <section id="architecture" order="2">
      <title>🏗️ Architecture & Design</title>
      <description>System architecture and core concepts</description>
      <pages>
        <page_ref>system-architecture</page_ref>
        <page_ref>core-concepts</page_ref>
        <page_ref>data-flow</page_ref>
      </pages>
    </section>
    
    <!-- Add 3-5 more logical sections based on the project -->
  </sections>
  
  <pages>
    <page id="overview">
      <title>Project Overview</title>
      <description>High-level introduction to the project, its goals, and key features</description>
      <importance>critical</importance>
      <estimated_length>800-1200 words</estimated_length>
      <content_focus>
        - What this project does and why it matters
        - Key features and benefits
        - Primary use cases and target scenarios
        - How it compares to alternatives (if relevant)
      </content_focus>
      <relevant_files>
        <file_path>README.md</file_path>
        <file_path>[main entry point file]</file_path>
        <file_path>[configuration file]</file_path>
      </relevant_files>
      <related_pages>
        <related>installation</related>
        <related>quick-start</related>
      </related_pages>
      <tags>
        <tag>overview</tag>
        <tag>introduction</tag>
        <tag>getting-started</tag>
      </tags>
    </page>
    
    <!-- Define 8-15 more pages with similar detail -->
  </pages>
</wiki_structure>
```

## 🎯 Page Planning Guidelines

### Importance Levels:
- **critical**: Must-read pages (Overview, Getting Started, Installation)
- **high**: Important for most users (Architecture, API Reference, Configuration)
- **medium**: Useful but not essential (Examples, Advanced Topics, Deployment)
- **low**: Supplementary content (FAQ, Troubleshooting, Contributing)

### Content Focus Areas:
- **Getting Started**: Installation, setup, first steps
- **Architecture**: System design, components, data flow
- **API/Reference**: Detailed technical documentation
- **Guides**: Step-by-step tutorials and examples
- **Configuration**: Settings, options, customization
- **Development**: Contributing, building, testing

### File Relevance:
- Identify the most important files for each page
- Include configuration files, main entry points, core modules
- Consider test files for understanding usage patterns
- Include documentation files (README, CHANGELOG, etc.)

## 📊 Quality Criteria

1. **Logical Organization**: Pages should follow a natural learning progression
2. **Comprehensive Coverage**: All major aspects of the project should be documented
3. **Balanced Scope**: Each page should have a focused, manageable scope
4. **Clear Relationships**: Related pages should be properly linked
5. **Practical Value**: Each page should provide actionable information

Generate a comprehensive wiki structure that serves as the foundation for excellent technical documentation.
"#,
            repo_info.title,
            repo_info.description,
            repo_info.languages.join(", "),
            repo_info.total_files,
            repo_info.has_api,
            file_tree
        )
    }

    /// Get language-specific instruction
    fn get_language_instruction(language: &str) -> &'static str {
        match language {
            "zh" => "IMPORTANT: Generate ALL content in Mandarin Chinese (中文). Use proper Chinese technical terminology and maintain professional tone.",
            "ja" => "IMPORTANT: Generate ALL content in Japanese (日本語). Use appropriate keigo and technical terminology.",
            "es" => "IMPORTANT: Generate ALL content in Spanish (Español). Use proper technical terminology and formal tone.",
            "fr" => "IMPORTANT: Generate ALL content in French (Français). Use appropriate technical vocabulary and formal style.",
            "ru" => "IMPORTANT: Generate ALL content in Russian (Русский). Use proper technical terminology and formal tone.",
            "ko" => "IMPORTANT: Generate ALL content in Korean (한국어). Use appropriate honorifics and technical terminology.",
            _ => "IMPORTANT: Generate ALL content in English. Use clear, professional technical language.",
        }
    }

    /// Get importance-specific guidance
    fn get_importance_guidance(importance: &ImportanceLevel) -> &'static str {
        match importance {
            ImportanceLevel::Critical => {
                "**CRITICAL PAGE**: This is essential documentation that most users will need. Provide comprehensive coverage (1000-1500 words) with multiple examples, best practices, and common pitfalls. Include extensive diagrams and detailed explanations."
            }
            ImportanceLevel::High => {
                "**HIGH PRIORITY**: This is important documentation for most users. Provide thorough coverage (800-1200 words) with key examples and practical guidance. Include relevant diagrams and clear explanations."
            }
            ImportanceLevel::Medium => {
                "**MEDIUM PRIORITY**: This is useful documentation for some users. Provide focused coverage (500-800 words) with essential information and examples. Include basic diagrams where helpful."
            }
            ImportanceLevel::Low => {
                "**LOW PRIORITY**: This is supplementary documentation. Provide concise coverage (300-500 words) focusing on essential information only. Keep diagrams simple and minimal."
            }
        }
    }

    /// Format file list for the prompt
    fn format_file_list(files: &[String], _repo_info: &RepositoryInfo) -> String {
        if files.is_empty() {
            return "- No specific files provided - use repository context".to_string();
        }

        files
            .iter()
            .map(|file| format!("- `{}`", file))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Research and analysis prompts for deeper understanding
pub struct ResearchPrompts;

impl ResearchPrompts {
    /// Generate questions for repository analysis
    pub fn create_analysis_questions(repo_info: &RepositoryInfo) -> String {
        format!(
            r#"Generate insightful analysis questions for the repository "{}".

## 📊 Repository Context
- **Name**: {}
- **Description**: {}
- **Languages**: {}
- **Has API**: {}

## 🔍 Generate Analysis Questions

Create 12-15 specific questions that would help create comprehensive documentation:

### Architecture Questions (3-4 questions)
1. What is the overall system architecture and how do components interact?
2. What design patterns are used and why?
3. How is the codebase organized and what are the key modules?

### Usage Questions (3-4 questions)  
4. Who is the primary target audience for this project?
5. What are the main use cases and user workflows?
6. How does this project compare to similar solutions?

### Technical Questions (3-4 questions)
7. What are the key dependencies and why were they chosen?
8. What are the performance characteristics and limitations?
9. How is error handling and logging implemented?

### Development Questions (3-4 questions)
10. How is the project structured for development and testing?
11. What are the coding conventions and best practices?
12. How is the build and deployment process organized?

Generate additional specific questions based on the project type, technology stack, and apparent complexity level.
"#,
            repo_info.title,
            repo_info.title,
            repo_info.description,
            repo_info.languages.join(", "),
            repo_info.has_api
        )
    }
}
