//! Wiki generation prompts
//!
//! This module contains all prompts used for wiki generation, inspired by DeepWiki's approach.

use crate::types::{RepositoryInfo, WikiConfig};

/// System prompt for wiki structure generation
pub const WIKI_STRUCTURE_SYSTEM_PROMPT: &str = r#"
You are an expert technical writer and software architect specializing in creating comprehensive documentation wikis for code repositories.

Your role is to analyze repository structure, understand the codebase architecture, and design logical wiki structures that help developers understand and work with the code effectively.

Key responsibilities:
- Analyze file structures and identify key components
- Understand project architecture and dependencies  
- Create logical page hierarchies and sections
- Identify files relevant to each documentation page
- Determine appropriate importance levels for content
- Design comprehensive yet navigable wiki structures

You have deep expertise in:
- Software architecture documentation
- API documentation best practices
- Developer onboarding workflows
- Technical writing and information architecture
- Multiple programming languages and frameworks
"#;

/// Template for wiki structure generation prompt
pub fn create_wiki_structure_prompt(
    repo_info: &RepositoryInfo,
    config: &WikiConfig,
    file_tree: &str,
) -> String {
    let language_instruction = match config.language.as_str() {
        "zh" => {
            "IMPORTANT: The wiki content will be generated in Mandarin Chinese (中文) language."
        }
        "ja" => "IMPORTANT: The wiki content will be generated in Japanese (日本語) language.",
        "es" => "IMPORTANT: The wiki content will be generated in Spanish (Español) language.",
        "fr" => "IMPORTANT: The wiki content will be generated in French (Français) language.",
        "ru" => "IMPORTANT: The wiki content will be generated in Russian (Русский) language.",
        "ko" => "IMPORTANT: The wiki content will be generated in Korean (한국어) language.",
        _ => "IMPORTANT: The wiki content will be generated in English language.",
    };

    let readme_section = if let Some(readme) = &repo_info.readme_content {
        format!(
            "2. The README file of the project:\n<readme>\n{}\n</readme>\n\n",
            readme
        )
    } else {
        "2. No README file found in the repository.\n\n".to_string()
    };

    let comprehensive_instruction = if config.comprehensive_view {
        r#"
Create a structured wiki with the following main sections:
- Overview (general information about the project)
- System Architecture (how the system is designed)
- Core Features (key functionality)
- Data Management/Flow: If applicable, how data is stored, processed, accessed, and managed (e.g., database schema, data pipelines, state management).
- Frontend Components (UI elements, if applicable.)
- Backend Systems (server-side components)
- Model Integration (AI model connections, if applicable)
- Deployment/Infrastructure (how to deploy, what's the infrastructure like)
- Extensibility and Customization: If the project architecture supports it, explain how to extend or customize its functionality (e.g., plugins, theming, custom modules, hooks).

Each section should contain relevant pages. For example, the "Frontend Components" section might include pages for "Home Page", "Repository Wiki Page", "Ask Component", etc.

Return your analysis in the following XML format:

<wiki_structure>
  <title>[Overall title for the wiki]</title>
  <description>[Brief description of the repository]</description>
  <sections>
    <section id="section-1">
      <title>[Section title]</title>
      <pages>
        <page_ref>page-1</page_ref>
        <page_ref>page-2</page_ref>
      </pages>
      <subsections>
        <section_ref>section-2</section_ref>
      </subsections>
    </section>
    <!-- More sections as needed -->
  </sections>
  <pages>
    <page id="page-1">
      <title>[Page title]</title>
      <description>[Brief description of what this page will cover]</description>
      <importance>high|medium|low</importance>
      <relevant_files>
        <file_path>[Path to a relevant file]</file_path>
        <!-- More file paths as needed -->
      </relevant_files>
      <related_pages>
        <related>page-2</related>
        <!-- More related page IDs as needed -->
      </related_pages>
      <parent_section>section-1</parent_section>
    </page>
    <!-- More pages as needed -->
  </pages>
</wiki_structure>
"#
    } else {
        r#"
Return your analysis in the following XML format:

<wiki_structure>
  <title>[Overall title for the wiki]</title>
  <description>[Brief description of the repository]</description>
  <pages>
    <page id="page-1">
      <title>[Page title]</title>
      <description>[Brief description of what this page will cover]</description>
      <importance>high|medium|low</importance>
      <relevant_files>
        <file_path>[Path to a relevant file]</file_path>
        <!-- More file paths as needed -->
      </relevant_files>
      <related_pages>
        <related>page-2</related>
        <!-- More related page IDs as needed -->
      </related_pages>
    </page>
    <!-- More pages as needed -->
  </pages>
</wiki_structure>
"#
    };

    let max_pages = if config.comprehensive_view {
        "8-12"
    } else {
        "4-6"
    };
    let wiki_type = if config.comprehensive_view {
        "comprehensive"
    } else {
        "concise"
    };

    format!(
        r#"Analyze this repository and create a wiki structure for it.

1. The complete file tree of the project:
<file_tree>
{}
</file_tree>

{}I want to create a wiki for this repository. Determine the most logical structure for a wiki based on the repository's content.

{}

When designing the wiki structure, include pages that would benefit from visual diagrams, such as:
- Architecture overviews
- Data flow descriptions
- Component relationships
- Process workflows
- State machines
- Class hierarchies

{}

IMPORTANT FORMATTING INSTRUCTIONS:
- Return ONLY the valid XML structure specified above
- DO NOT wrap the XML in markdown code blocks (no ``` or ```xml)
- DO NOT include any explanation text before or after the XML
- Ensure the XML is properly formatted and valid
- Start directly with <wiki_structure> and end with </wiki_structure>

IMPORTANT:
1. Create {} pages that would make a {} wiki for this repository
2. Each page should focus on a specific aspect of the codebase (e.g., architecture, key features, setup)
3. The relevant_files should be actual files from the repository that would be used to generate that page
4. Return ONLY valid XML with the structure specified above, with no markdown code block delimiters"#,
        file_tree,
        readme_section,
        language_instruction,
        comprehensive_instruction,
        max_pages,
        wiki_type
    )
}

/// System prompt for wiki page content generation
pub const WIKI_PAGE_CONTENT_SYSTEM_PROMPT: &str = r#"
You are an expert technical writer specializing in creating comprehensive, accurate, and well-structured documentation for software projects.

Your role is to generate detailed wiki page content based on:
- Repository code analysis
- File structure understanding
- Code functionality and architecture
- Best practices in technical documentation

Key responsibilities:
- Write clear, comprehensive technical documentation
- Explain complex concepts in an accessible way
- Provide practical examples and code snippets
- Structure content with proper headings and organization
- Include relevant diagrams and visualizations when helpful
- Maintain consistency in tone and style

You have expertise in:
- Multiple programming languages and frameworks
- Software architecture patterns
- API documentation
- Developer onboarding processes
- Technical writing best practices
- Markdown formatting and structure
"#;

/// Template for wiki page content generation prompt
pub fn create_page_content_prompt(
    page_title: &str,
    page_description: &str,
    relevant_files: &[String],
    repo_info: &RepositoryInfo,
    config: &WikiConfig,
) -> String {
    let language_instruction = match config.language.as_str() {
        "zh" => "IMPORTANT: Write all content in Mandarin Chinese (中文).",
        "ja" => "IMPORTANT: Write all content in Japanese (日本語).",
        "es" => "IMPORTANT: Write all content in Spanish (Español).",
        "fr" => "IMPORTANT: Write all content in French (Français).",
        "ru" => "IMPORTANT: Write all content in Russian (Русский).",
        "ko" => "IMPORTANT: Write all content in Korean (한국어).",
        _ => "IMPORTANT: Write all content in English.",
    };

    let files_context = if !relevant_files.is_empty() {
        format!(
            "Focus on these specific files for this page:\n{}\n\n",
            relevant_files
                .iter()
                .map(|f| format!("- {}", f))
                .collect::<Vec<_>>()
                .join("\n")
        )
    } else {
        String::new()
    };

    let diagram_instruction = if config.include_diagrams {
        "\n- Include Mermaid diagrams where they would help explain concepts (use ```mermaid code blocks)"
    } else {
        ""
    };

    format!(
        r#"Generate comprehensive technical documentation for the wiki page: "{}"

Page Description: {}

Repository Context:
- Project: {}
- Languages: {}
- Type: {}

{}{}

Requirements:
1. Write detailed, accurate technical documentation
2. Use proper Markdown formatting with clear headings
3. Include code examples where relevant
4. Explain concepts thoroughly but concisely
5. Structure content logically with:
   - Overview/Introduction
   - Key Concepts
   - Implementation Details
   - Code Examples (if applicable)
   - Best Practices
   - Related Information{}

Content Guidelines:
- Start with a clear overview of what this page covers
- Use bullet points and numbered lists for clarity
- Include code snippets with proper syntax highlighting
- Add cross-references to related concepts
- End with practical next steps or related resources
- Maintain professional, technical tone
- Ensure accuracy and completeness

Generate the complete Markdown content for this wiki page:"#,
        page_title,
        page_description,
        repo_info.title,
        repo_info.languages.join(", "),
        if repo_info.has_api {
            "API/Service"
        } else {
            "Library/Application"
        },
        files_context,
        language_instruction,
        diagram_instruction
    )
}

/// System prompt for RAG-enhanced content generation
pub const RAG_WIKI_SYSTEM_PROMPT: &str = r#"
You are an expert technical writer and code analyst creating comprehensive wiki documentation for software repositories.

You will receive:
1. A specific documentation request
2. Relevant code context from the repository
3. File structure and project information

Your role is to:
- Analyze the provided code context thoroughly
- Generate accurate, detailed technical documentation
- Explain code functionality and architecture
- Provide practical examples and usage instructions
- Structure content for maximum developer utility

Key principles:
- Base all content on the actual code provided in context
- Explain complex concepts clearly and systematically
- Include relevant code examples with explanations
- Maintain consistency with project conventions
- Focus on practical, actionable information
- Use proper technical terminology

Response format:
- Use clear Markdown formatting
- Structure with appropriate headings
- Include code blocks with syntax highlighting
- Add diagrams when they clarify concepts
- Provide cross-references to related components
"#;
