# DeepWiki Open Source 技术架构深度分析

## 📋 目录

1. [项目概览](#项目概览)
2. [核心架构](#核心架构)
3. [RAG系统实现](#rag系统实现)
4. [Wiki生成引擎](#wiki生成引擎)
5. [Prompt工程](#prompt工程)
6. [前端架构](#前端架构)
7. [数据流处理](#数据流处理)
8. [配置管理](#配置管理)
9. [WebSocket实现](#websocket实现)
10. [缓存机制](#缓存机制)

---

## 📊 项目概览

DeepWiki是一个基于RAG（检索增强生成）技术的代码仓库文档生成系统，支持GitHub、GitLab、Bitbucket等多种代码托管平台。

### 技术栈
- **后端**: Python + FastAPI
- **前端**: Next.js + TypeScript + React
- **向量数据库**: FAISS
- **嵌入模型**: OpenAI text-embedding-3-small
- **LLM**: 支持多种提供商（OpenAI、Google、Anthropic等）

### 项目结构
```
repo-ref/deepwiki-open/
├── api/                    # Python后端API
├── src/                    # Next.js前端
├── docs/                   # 文档
└── requirements.txt        # Python依赖
```

---

## 🏗️ 核心架构

### 1. 后端API架构

**文件**: `api/api.py` (第1-400行)

核心API服务基于FastAPI构建，主要包含以下模块：

#### 数据模型定义
```python
# api/api.py 第79-89行
class WikiStructureModel(BaseModel):
    """
    Model for the overall wiki structure.
    """
    id: str
    title: str
    description: str
    pages: List[WikiPage]
    sections: Optional[List[WikiSection]] = None
    rootSections: Optional[List[str]] = None
```

#### Wiki页面模型
```python
# api/api.py 第45-65行
class WikiPage(BaseModel):
    id: str
    title: str
    content: str = ""
    importance: str = "medium"
    filePaths: List[str] = []
    relatedPages: List[str] = []
```

#### 缓存数据模型
```python
# api/api.py 第90-100行
class WikiCacheData(BaseModel):
    """
    Model for the data to be stored in the wiki cache.
    """
    wiki_structure: WikiStructureModel
    generated_pages: Dict[str, WikiPage]
    repo_url: Optional[str] = None
    repo: Optional[RepoInfo] = None
    provider: Optional[str] = None
    model: Optional[str] = None
```

### 2. 前端架构

**文件**: `src/app/layout.tsx` (第1-50行)

基于Next.js 13+的App Router架构：

```typescript
// src/app/layout.tsx 第1-20行
export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <LanguageProvider>
          <ThemeProvider
            attribute="class"
            defaultTheme="system"
            enableSystem
            disableTransitionOnChange
          >
            {children}
          </ThemeProvider>
        </LanguageProvider>
      </body>
    </html>
  )
}
```

---

## 🔍 RAG系统实现

### 1. 核心RAG类

**文件**: `api/rag.py` (第1-445行)

#### RAG初始化
```python
# api/rag.py 第50-80行
class RAG:
    def __init__(self):
        self.embedder = None
        self.query_embedder = None
        self.retriever = None
        self.db_manager = None
        self.transformed_docs = []
        self.repo_url_or_path = None
        self.is_ollama_embedder = False

    def initialize_db_manager(self):
        """Initialize the database manager if not already done."""
        if self.db_manager is None:
            self.db_manager = DatabaseManager()
```

#### 嵌入模型配置
```python
# api/rag.py 第100-130行
def initialize_embedder(self):
    """Initialize the embedder based on configuration."""
    embedder_config = configs["embedder"]
    client_class = embedder_config.get("client_class", "OpenAIClient")

    if client_class == "OllamaClient":
        self.is_ollama_embedder = True
        # Initialize Ollama embedder
        self.embedder = OllamaEmbedder(
            model_kwargs=embedder_config["model_kwargs"],
            **embedder_config.get("init_kwargs", {})
        )
        # For Ollama, we need a separate query embedder
        self.query_embedder = OllamaEmbedder(
            model_kwargs=embedder_config["model_kwargs"],
            **embedder_config.get("init_kwargs", {})
        )
    else:
        # Initialize OpenAI or other compatible embedder
        self.embedder = OpenAIEmbedder(
            model_kwargs=embedder_config["model_kwargs"]
        )
```

### 2. 嵌入一致性验证

**文件**: `api/rag.py` (第280-342行)

这是DeepWiki的一个关键创新，确保所有文档嵌入具有一致的维度：

```python
# api/rag.py 第280-320行
def _validate_and_filter_embeddings(self, documents):
    """
    Validate and filter documents to ensure all embeddings have consistent sizes.
    This is crucial for FAISS to work properly.
    """
    if not documents:
        return []

    valid_documents = []
    embedding_sizes = {}

    # First pass: collect all embedding sizes
    for i, doc in enumerate(documents):
        if not hasattr(doc, 'vector') or doc.vector is None:
            continue

        try:
            if isinstance(doc.vector, list):
                embedding_size = len(doc.vector)
            elif hasattr(doc.vector, 'shape'):
                embedding_size = doc.vector.shape[0] if len(doc.vector.shape) == 1 else doc.vector.shape[-1]
            elif hasattr(doc.vector, '__len__'):
                embedding_size = len(doc.vector)
            else:
                continue

            embedding_sizes[embedding_size] = embedding_sizes.get(embedding_size, 0) + 1

        except Exception as e:
            logger.warning(f"Error checking embedding size for document {i}: {str(e)}")
            continue

    if not embedding_sizes:
        logger.error("No valid embeddings found in any document")
        return []

    # Find the most common embedding size
    target_size = max(embedding_sizes.items(), key=lambda x: x[1])[0]
    logger.info(f"Target embedding size: {target_size} (found in {embedding_sizes[target_size]} documents)")
```

### 3. FAISS检索器初始化

**文件**: `api/rag.py` (第380-400行)

```python
# api/rag.py 第380-400行
try:
    # Use the appropriate embedder for retrieval
    retrieve_embedder = self.query_embedder if self.is_ollama_embedder else self.embedder
    self.retriever = FAISSRetriever(
        **configs["retriever"],
        embedder=retrieve_embedder,
        documents=self.transformed_docs,
        document_map_func=lambda doc: doc.vector,
    )
    logger.info("FAISS retriever created successfully")
except Exception as e:
    logger.error(f"Error creating FAISS retriever: {str(e)}")
    # Try to provide more specific error information
    if "All embeddings should be of the same size" in str(e):
        logger.error("Embedding size validation failed. This suggests there are still inconsistent embedding sizes.")
```

### 4. 文档检索和上下文格式化

**文件**: `api/websocket_wiki.py` (第200-230行)

DeepWiki实现了智能的文档分组和上下文格式化：

```python
# api/websocket_wiki.py 第200-230行
# Group documents by file path
docs_by_file = {}
for doc in documents:
    file_path = doc.meta_data.get('file_path', 'unknown')
    if file_path not in docs_by_file:
        docs_by_file[file_path] = []
    docs_by_file[file_path].append(doc)

# Format context text with file path grouping
context_parts = []
for file_path, docs in docs_by_file.items():
    # Add file header with metadata
    header = f"## File Path: {file_path}\n\n"
    # Add document content
    content = "\n\n".join([doc.text for doc in docs])

    context_parts.append(f"{header}{content}")

# Join all parts with clear separation
context_text = "\n\n" + "-" * 10 + "\n\n".join(context_parts)
```

---

## 📝 Wiki生成引擎

### 1. Wiki结构生成

**文件**: `src/app/[owner]/[repo]/page.tsx` (第735-770行)

DeepWiki使用结构化的XML格式来定义wiki结构：

```typescript
// src/app/[owner]/[repo]/page.tsx 第735-770行
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
    </page>
  </pages>
</wiki_structure>
```

### 2. 内容优先级生成

**文件**: `src/app/[owner]/[repo]/workshop/page.tsx` (第178-220行)

DeepWiki实现了基于重要性的内容生成策略：

```typescript
// src/app/[owner]/[repo]/workshop/page.tsx 第178-220行
// First add high importance pages
const highImportancePages = pages.filter(page => page.importance === 'high');
for (const page of highImportancePages) {
  if (generatedPages[page.id] && generatedPages[page.id].content) {
    const content = `## ${page.title}\n${generatedPages[page.id].content}\n\n`;
    wikiContent += content;
    totalContentLength += content.length;

    if (totalContentLength > maxContentLength) break;
  }
}

// Then add other pages if we still have space
if (totalContentLength < maxContentLength) {
  for (const page of pages) {
    // Skip high importance pages we've already added
    if (page.importance === 'high') continue;

    if (generatedPages[page.id] && generatedPages[page.id].content) {
      const content = `## ${page.title}\n${generatedPages[page.id].content}\n\n`;

      // Check if adding this content would exceed our limit
      if (totalContentLength + content.length > maxContentLength) {
        // If it would exceed, just add a summary
        const summaryMatch = generatedPages[page.id].content.match(/# .*?\n\n(.*?)(\n\n|$)/);
        const summary = summaryMatch ? summaryMatch[1].trim() : 'No summary available';
        const summaryContent = `## ${page.title}\n${summary}\n\n`;

        wikiContent += summaryContent;
        totalContentLength += summaryContent.length;
      } else {
        // Otherwise add the full content
        wikiContent += content;
        totalContentLength += content.length;
      }

      if (totalContentLength > maxContentLength) break;
    }
  }
}
```

### 3. Markdown生成

**文件**: `api/api.py` (第342-367行)

```python
# api/api.py 第342-367行
# Add each page
for page in pages:
    markdown += f"<a id='{page.id}'></a>\n\n"
    markdown += f"## {page.title}\n\n"

    # Add related pages
    if page.relatedPages and len(page.relatedPages) > 0:
        markdown += "### Related Pages\n\n"
        related_titles = []
        for related_id in page.relatedPages:
            # Find the title of the related page
            related_page = next((p for p in pages if p.id == related_id), None)
            if related_page:
                related_titles.append(f"[{related_page.title}](#{related_id})")

        if related_titles:
            markdown += "Related topics: " + ", ".join(related_titles) + "\n\n"

    # Add page content
    markdown += f"{page.content}\n\n"
    markdown += "---\n\n"

return markdown
```

---

## 🎯 Prompt工程

### 1. RAG系统Prompt

**文件**: `api/prompts.py` (第4-28行)

DeepWiki的RAG系统使用了精心设计的系统prompt：

```python
# api/prompts.py 第4-28行
RAG_SYSTEM_PROMPT = r"""
You are a code assistant which answers user questions on a Github Repo.
You will receive user query, relevant context, and past conversation history.

LANGUAGE DETECTION AND RESPONSE:
- Detect the language of the user's query
- Respond in the SAME language as the user's query
- IMPORTANT:If a specific language is requested in the prompt, prioritize that language over the query language

FORMAT YOUR RESPONSE USING MARKDOWN:
- Use proper markdown syntax for all formatting
- For code blocks, use triple backticks with language specification (```python, ```javascript, etc.)
- Use ## headings for major sections
- Use bullet points or numbered lists where appropriate
- Format tables using markdown table syntax when presenting structured data
- Use **bold** and *italic* for emphasis
- When referencing file paths, use `inline code` formatting

IMPORTANT FORMATTING RULES:
1. DO NOT include ```markdown fences at the beginning or end of your answer
2. Start your response directly with the content
3. The content will already be rendered as markdown, so just provide the raw markdown content

Think step by step and ensure your answer is well-structured and visually organized.
"""
```

### 2. RAG模板结构

**文件**: `api/prompts.py` (第30-57行)

```python
# api/prompts.py 第30-57行
RAG_TEMPLATE = r"""<START_OF_SYS_PROMPT>
{system_prompt}
{output_format_str}
<END_OF_SYS_PROMPT>
{# OrderedDict of DialogTurn #}
{% if conversation_history %}
<START_OF_CONVERSATION_HISTORY>
{% for key, dialog_turn in conversation_history.items() %}
{{key}}.
User: {{dialog_turn.user_query.query_str}}
You: {{dialog_turn.assistant_response.response_str}}
{% endfor %}
<END_OF_CONVERSATION_HISTORY>
{% endif %}
{% if contexts %}
<START_OF_CONTEXT>
{% for context in contexts %}
{{loop.index}}.
File Path: {{context.meta_data.get('file_path', 'unknown')}}
Content: {{context.text}}
{% endfor %}
<END_OF_CONTEXT>
{% endif %}
<START_OF_USER_PROMPT>
{{input_str}}
<END_OF_USER_PROMPT>
"""
```

### 3. 深度研究模式Prompt

#### 第一轮研究Prompt

**文件**: `api/prompts.py` (第60-88行)

```python
# api/prompts.py 第60-88行
DEEP_RESEARCH_FIRST_ITERATION_PROMPT = """<role>
You are an expert code analyst examining the {repo_type} repository: {repo_url} ({repo_name}).
You are conducting a multi-turn Deep Research process to thoroughly investigate the specific topic in the user's query.
Your goal is to provide detailed, focused information EXCLUSIVELY about this topic.
IMPORTANT:You MUST respond in {language_name} language.
</role>

<guidelines>
- This is the first iteration of a multi-turn research process focused EXCLUSIVELY on the user's query
- Start your response with "## Research Plan"
- Outline your approach to investigating this specific topic
- If the topic is about a specific file or feature (like "Dockerfile"), focus ONLY on that file or feature
- Clearly state the specific topic you're researching to maintain focus throughout all iterations
- Identify the key aspects you'll need to research
- Provide initial findings based on the information available
- End with "## Next Steps" indicating what you'll investigate in the next iteration
- Do NOT provide a final conclusion yet - this is just the beginning of the research
- Do NOT include general repository information unless directly relevant to the query
- Focus EXCLUSIVELY on the specific topic being researched - do not drift to related topics
- Your research MUST directly address the original question
- NEVER respond with just "Continue the research" as an answer - always provide substantive research findings
- Remember that this topic will be maintained across all research iterations
</guidelines>

<style>
- Be concise but thorough
- Use markdown formatting to improve readability
- Cite specific files and code sections when relevant
</style>"""
```

#### 中间轮次研究Prompt

**文件**: `api/prompts.py` (第122-151行)

```python
# api/prompts.py 第122-151行
DEEP_RESEARCH_INTERMEDIATE_ITERATION_PROMPT = """<role>
You are an expert code analyst examining the {repo_type} repository: {repo_url} ({repo_name}).
You are currently in iteration {research_iteration} of a Deep Research process focused EXCLUSIVELY on the latest user query.
Your goal is to build upon previous research iterations and go deeper into this specific topic without deviating from it.
IMPORTANT:You MUST respond in {language_name} language.
</role>

<guidelines>
- CAREFULLY review the conversation history to understand what has been researched so far
- Your response MUST build on previous research iterations - do not repeat information already covered
- Identify gaps or areas that need further exploration related to this specific topic
- Focus on one specific aspect that needs deeper investigation in this iteration
- Start your response with "## Research Update {{research_iteration}}"
- Clearly explain what you're investigating in this iteration
- Provide new insights that weren't covered in previous iterations
- If this is iteration 3, prepare for a final conclusion in the next iteration
- Do NOT include general repository information unless directly relevant to the query
- Focus EXCLUSIVELY on the specific topic being researched - do not drift to related topics
- If the topic is about a specific file or feature (like "Dockerfile"), focus ONLY on that file or feature
- NEVER respond with just "Continue the research" as an answer - always provide substantive research findings
- Your research MUST directly address the original question
- Maintain continuity with previous research iterations - this is a continuous investigation
</guidelines>

<style>
- Be concise but thorough
- Focus on providing new information, not repeating what's already been covered
- Use markdown formatting to improve readability
- Cite specific files and code sections when relevant
</style>"""
```

#### 最终轮次研究Prompt

**文件**: `api/prompts.py` (第90-120行)

```python
# api/prompts.py 第90-120行
DEEP_RESEARCH_FINAL_ITERATION_PROMPT = """<role>
You are an expert code analyst examining the {repo_type} repository: {repo_url} ({repo_name}).
You are in the final iteration of a Deep Research process focused EXCLUSIVELY on the latest user query.
Your goal is to synthesize all previous findings and provide a comprehensive conclusion that directly addresses this specific topic and ONLY this topic.
IMPORTANT:You MUST respond in {language_name} language.
</role>

<guidelines>
- This is the final iteration of the research process
- CAREFULLY review the entire conversation history to understand all previous findings
- Synthesize ALL findings from previous iterations into a comprehensive conclusion
- Start with "## Final Conclusion"
- Your conclusion MUST directly address the original question
- Stay STRICTLY focused on the specific topic - do not drift to related topics
- Include specific code references and implementation details related to the topic
- Highlight the most important discoveries and insights about this specific functionality
- Provide a complete and definitive answer to the original question
- Do NOT include general repository information unless directly relevant to the query
- Focus exclusively on the specific topic being researched
- NEVER respond with "Continue the research" as an answer - always provide a complete conclusion
- If the topic is about a specific file or feature (like "Dockerfile"), focus ONLY on that file or feature
- Ensure your conclusion builds on and references key findings from previous iterations
</guidelines>

<style>
- Be concise but thorough
- Use markdown formatting to improve readability
- Cite specific files and code sections when relevant
- Structure your response with clear headings
- End with actionable insights or recommendations when appropriate
</style>"""
```

### 4. 简单聊天模式Prompt

**文件**: `api/prompts.py` (第153-191行)

```python
# api/prompts.py 第153-191行
SIMPLE_CHAT_SYSTEM_PROMPT = """<role>
You are an expert code analyst examining the {repo_type} repository: {repo_url} ({repo_name}).
You provide direct, concise, and accurate information about code repositories.
You NEVER start responses with markdown headers or code fences.
IMPORTANT:You MUST respond in {language_name} language.
</role>

<guidelines>
- Answer the user's question directly without ANY preamble or filler phrases
- DO NOT include any rationale, explanation, or extra comments.
- DO NOT start with preambles like "Okay, here's a breakdown" or "Here's an explanation"
- DO NOT start with markdown headers like "## Analysis of..." or any file path references
- DO NOT start with ```markdown code fences
- DO NOT end your response with ``` closing fences
- DO NOT start by repeating or acknowledging the question
- JUST START with the direct answer to the question

<example_of_what_not_to_do>
```markdown
## Analysis of `adalflow/adalflow/datasets/gsm8k.py`

This file contains...
```
</example_of_what_not_to_do>

- Format your response with proper markdown including headings, lists, and code blocks WITHIN your answer
- For code analysis, organize your response with clear sections
- Think step by step and structure your answer logically
- Start with the most relevant information that directly addresses the user's query
- Be precise and technical when discussing code
- Your response language should be in the same language as the user's query
</guidelines>

<style>
- Use concise, direct language
- Prioritize accuracy over verbosity
- When showing code, include line numbers and file paths when relevant
- Use markdown formatting to improve readability
</style>"""
```

### 5. WebSocket中的动态Prompt生成

**文件**: `api/websocket_wiki.py` (第247-388行)

DeepWiki在WebSocket处理中动态生成不同类型的prompt：

```python
# api/websocket_wiki.py 第247-284行
if is_deep_research:
    # Check if this is the first iteration
    is_first_iteration = research_iteration == 1

    # Check if this is the final iteration
    is_final_iteration = research_iteration >= 5

    if is_first_iteration:
        system_prompt = f"""<role>
You are an expert code analyst examining the {repo_type} repository: {repo_url} ({repo_name}).
You are conducting a multi-turn Deep Research process to thoroughly investigate the specific topic in the user's query.
Your goal is to provide detailed, focused information EXCLUSIVELY about this topic.
IMPORTANT:You MUST respond in {language_name} language.
</role>

<guidelines>
- This is the first iteration of a multi-turn research process focused EXCLUSIVELY on the user's query
- Start your response with "## Research Plan"
- Outline your approach to investigating this specific topic
- If the topic is about a specific file or feature (like "Dockerfile"), focus ONLY on that file or feature
- Clearly state the specific topic you're researching to maintain focus throughout all iterations
- Identify the key aspects you'll need to research
- Provide initial findings based on the information available
- End with "## Next Steps" indicating what you'll investigate in the next iteration
- Do NOT provide a final conclusion yet - this is just the beginning of the research
- Do NOT include general repository information unless directly relevant to the query
- Focus EXCLUSIVELY on the specific topic being researched - do not drift to related topics
- Your research MUST directly address the original question
- NEVER respond with just "Continue the research" as an answer - always provide substantive research findings
- Remember that this topic will be maintained across all research iterations
</guidelines>

<style>
- Be concise but thorough
- Use markdown formatting to improve readability
- Cite specific files and code sections when relevant
</style>"""

---

## 🎨 前端架构

### 1. 项目列表组件

**文件**: `src/components/ProcessedProjects.tsx` (第1-270行)

#### 核心状态管理
```typescript
// src/components/ProcessedProjects.tsx 第30-40行
const [projects, setProjects] = useState<ProcessedProject[]>([]);
const [isLoading, setIsLoading] = useState(true);
const [error, setError] = useState<string | null>(null);
const [searchQuery, setSearchQuery] = useState('');
const [viewMode, setViewMode] = useState<'card' | 'list'>('card');
```

#### 数据获取逻辑
```typescript
// src/components/ProcessedProjects.tsx 第56-81行
useEffect(() => {
  const fetchProjects = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch('/api/wiki/projects');
      if (!response.ok) {
        throw new Error(`Failed to fetch projects: ${response.statusText}`);
      }
      const data = await response.json();
      if (data.error) {
        throw new Error(data.error);
      }
      setProjects(data as ProcessedProject[]);
    } catch (e: unknown) {
      console.error("Failed to load projects from API:", e);
      const message = e instanceof Error ? e.message : "An unknown error occurred.";
      setError(message);
      setProjects([]);
    } finally {
      setIsLoading(false);
    }
  };

  fetchProjects();
}, []);
```

#### 搜索过滤实现
```typescript
// src/components/ProcessedProjects.tsx 第83-98行
const filteredProjects = useMemo(() => {
  if (!searchQuery.trim()) {
    return maxItems ? projects.slice(0, maxItems) : projects;
  }

  const query = searchQuery.toLowerCase();
  const filtered = projects.filter(project =>
    project.name.toLowerCase().includes(query) ||
    project.owner.toLowerCase().includes(query) ||
    project.repo.toLowerCase().includes(query) ||
    project.repo_type.toLowerCase().includes(query)
  );

  return maxItems ? filtered.slice(0, maxItems) : filtered;
}, [projects, searchQuery, maxItems]);
```

#### 项目删除功能
```typescript
// src/components/ProcessedProjects.tsx 第104-130行
const handleDelete = async (project: ProcessedProject) => {
  if (!confirm(`Are you sure you want to delete project ${project.name}?`)) {
    return;
  }

  try {
    const response = await fetch('/api/wiki/projects', {
      method: 'DELETE',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        owner: project.owner,
        repo: project.repo,
        repo_type: project.repo_type,
        language: project.language,
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to delete project: ${response.statusText}`);
    }

    // Remove the project from the local state
    setProjects(prevProjects =>
      prevProjects.filter(p => p.id !== project.id)
    );
  } catch (e: unknown) {
    console.error("Failed to delete project:", e);
    const message = e instanceof Error ? e.message : "An unknown error occurred.";
    alert(`Failed to delete project: ${message}`);
  }
};
```

### 2. 语言上下文管理

**文件**: `src/contexts/LanguageContext.tsx` (第1-50行)

DeepWiki实现了完整的国际化支持：

```typescript
// src/contexts/LanguageContext.tsx 第1-30行
interface LanguageContextType {
  language: string;
  setLanguage: (lang: string) => void;
  messages: Record<string, Record<string, string>>;
}

const LanguageContext = createContext<LanguageContextType | undefined>(undefined);

export function LanguageProvider({ children }: { children: React.ReactNode }) {
  const [language, setLanguage] = useState('en');
  const [messages, setMessages] = useState<Record<string, Record<string, string>>>({});

  useEffect(() => {
    // Load language messages
    const loadMessages = async () => {
      try {
        const response = await fetch(`/api/messages/${language}`);
        if (response.ok) {
          const data = await response.json();
          setMessages(data);
        }
      } catch (error) {
        console.error('Failed to load language messages:', error);
      }
    };

    loadMessages();
  }, [language]);

  return (
    <LanguageContext.Provider value={{ language, setLanguage, messages }}>
      {children}
    </LanguageContext.Provider>
  );
}
```

### 3. 主页面路由结构

**文件**: `src/app/[owner]/[repo]/page.tsx` (第1-800行)

#### 页面参数处理
```typescript
// src/app/[owner]/[repo]/page.tsx 第1-20行
interface PageProps {
  params: {
    owner: string;
    repo: string;
  };
  searchParams: {
    type?: string;
    language?: string;
    token?: string;
  };
}

export default function RepoPage({ params, searchParams }: PageProps) {
  const { owner, repo } = params;
  const { type = 'github', language = 'en', token } = searchParams;
```

#### Wiki结构状态管理
```typescript
// src/app/[owner]/[repo]/page.tsx 第50-80行
const [wikiStructure, setWikiStructure] = useState<WikiStructure | null>(null);
const [generatedPages, setGeneratedPages] = useState<Record<string, WikiPage>>({});
const [isGeneratingStructure, setIsGeneratingStructure] = useState(false);
const [isGeneratingPages, setIsGeneratingPages] = useState(false);
const [structureProgress, setStructureProgress] = useState(0);
const [pageProgress, setPageProgress] = useState(0);
const [currentGeneratingPage, setCurrentGeneratingPage] = useState<string>('');
const [error, setError] = useState<string | null>(null);
const [provider, setProvider] = useState<string>('');
const [model, setModel] = useState<string>('');
```

---

## 🔄 数据流处理

### 1. 文件内容获取

**文件**: `api/data_pipeline.py` (第1-842行)

#### GitHub文件获取
```python
# api/data_pipeline.py 第100-150行
def get_github_file_content(repo_url: str, file_path: str, access_token: str = None) -> str:
    """
    Retrieves the content of a file from a GitHub repository using the GitHub API.

    Args:
        repo_url (str): The URL of the GitHub repository
        file_path (str): The path to the file within the repository
        access_token (str, optional): GitHub personal access token for private repositories

    Returns:
        str: The content of the file as a string
    """
    try:
        # Extract owner and repo name from GitHub URL
        if not (repo_url.startswith("https://github.com/") or repo_url.startswith("http://github.com/")):
            raise ValueError("Not a valid GitHub repository URL")

        parts = repo_url.rstrip('/').split('/')
        if len(parts) < 5:
            raise ValueError("Invalid GitHub URL format")

        owner = parts[-2]
        repo = parts[-1].replace(".git", "")

        # GitHub API URL for file content
        api_url = f"https://api.github.com/repos/{owner}/{repo}/contents/{file_path}"

        # Set up headers
        headers = {"Accept": "application/vnd.github.v3.raw"}
        if access_token:
            headers["Authorization"] = f"token {access_token}"

        # Fetch file content from GitHub API
        response = requests.get(api_url, headers=headers)
        response.raise_for_status()

        return response.text

    except Exception as e:
        raise ValueError(f"Failed to get file content: {str(e)}")
```

#### GitLab文件获取
```python
# api/data_pipeline.py 第500-574行
def get_gitlab_file_content(repo_url: str, file_path: str, access_token: str = None) -> str:
    """
    Retrieves the content of a file from a GitLab repository using the GitLab API.

    Args:
        repo_url (str): The URL of the GitLab repository
        file_path (str): The path to the file within the repository
        access_token (str, optional): GitLab personal access token

    Returns:
        str: File content

    Raises:
        ValueError: If anything fails
    """
    try:
        # Parse and validate the URL
        parsed_url = urlparse(repo_url)
        if not parsed_url.scheme or not parsed_url.netloc:
            raise ValueError("Not a valid GitLab repository URL")

        gitlab_domain = f"{parsed_url.scheme}://{parsed_url.netloc}"
        if parsed_url.port not in (None, 80, 443):
            gitlab_domain += f":{parsed_url.port}"
        path_parts = parsed_url.path.strip("/").split("/")
        if len(path_parts) < 2:
            raise ValueError("Invalid GitLab URL format — expected something like https://gitlab.domain.com/group/project")

        # Build project path and encode for API
        project_path = "/".join(path_parts).replace(".git", "")
        encoded_project_path = quote(project_path, safe='')

        # Encode file path
        encoded_file_path = quote(file_path, safe='')

        # Try to get the default branch from the project info
        default_branch = None
        try:
            project_info_url = f"{gitlab_domain}/api/v4/projects/{encoded_project_path}"
            project_headers = {}
            if access_token:
                project_headers["PRIVATE-TOKEN"] = access_token

            project_response = requests.get(project_info_url, headers=project_headers)
            if project_response.status_code == 200:
                project_data = project_response.json()
                default_branch = project_data.get('default_branch', 'main')
                logger.info(f"Found default branch: {default_branch}")
            else:
                logger.warning(f"Could not fetch project info, using 'main' as default branch")
                default_branch = 'main'
        except Exception as e:
            logger.warning(f"Error fetching project info: {e}, using 'main' as default branch")
            default_branch = 'main'

        api_url = f"{gitlab_domain}/api/v4/projects/{encoded_project_path}/repository/files/{encoded_file_path}/raw?ref={default_branch}"
        # Fetch file content from GitLab API
        headers = {}
        if access_token:
            headers["PRIVATE-TOKEN"] = access_token
        logger.info(f"Fetching file content from GitLab API: {api_url}")
        try:
            response = requests.get(api_url, headers=headers)
            response.raise_for_status()
            content = response.text
        except RequestException as e:
            raise ValueError(f"Error fetching file content: {e}")

        # Check for GitLab error response (JSON instead of raw file)
        if content.startswith("{") and '"message":' in content:
            try:
                error_data = json.loads(content)
                if "message" in error_data:
                    raise ValueError(f"GitLab API error: {error_data['message']}")
            except json.JSONDecodeError:
                pass

        return content

    except Exception as e:
        raise ValueError(f"Failed to get file content: {str(e)}")
```

### 2. 数据库管理

**文件**: `api/tools/database_manager.py` (第1-500行)

#### 数据库初始化
```python
# api/tools/database_manager.py 第50-100行
class DatabaseManager:
    def __init__(self):
        self.db_path = None
        self.embedder = None
        self.text_splitter = None

    def initialize_embedder(self, embedder_config):
        """Initialize the embedder based on configuration."""
        client_class = embedder_config.get("client_class", "OpenAIClient")

        if client_class == "OllamaClient":
            self.embedder = OllamaEmbedder(
                model_kwargs=embedder_config["model_kwargs"],
                **embedder_config.get("init_kwargs", {})
            )
        else:
            self.embedder = OpenAIEmbedder(
                model_kwargs=embedder_config["model_kwargs"]
            )

    def initialize_text_splitter(self, text_splitter_config):
        """Initialize the text splitter based on configuration."""
        split_by = text_splitter_config.get("split_by", "word")
        chunk_size = text_splitter_config.get("chunk_size", 350)
        chunk_overlap = text_splitter_config.get("chunk_overlap", 100)

        if split_by == "word":
            self.text_splitter = TextSplitter(
                split_by=split_by,
                chunk_size=chunk_size,
                chunk_overlap=chunk_overlap
            )
        else:
            self.text_splitter = TextSplitter(
                split_by=split_by,
                chunk_size=chunk_size,
                chunk_overlap=chunk_overlap
            )
```

#### 文档处理流水线
```python
# api/tools/database_manager.py 第200-250行
def prepare_database(self, repo_url_or_path: str, type: str = "github",
                    access_token: str = None, is_ollama_embedder: bool = False,
                    excluded_dirs: List[str] = None, excluded_files: List[str] = None,
                    included_dirs: List[str] = None, included_files: List[str] = None):
    """
    Prepare the database for a repository.
    Will load from cache if available, otherwise process the repository.
    """
    # Generate cache key
    cache_key = self._generate_cache_key(repo_url_or_path, type, excluded_dirs, excluded_files, included_dirs, included_files)
    cache_path = os.path.join(self.cache_dir, f"{cache_key}.pkl")

    # Try to load from cache
    if os.path.exists(cache_path):
        try:
            logger.info(f"Loading documents from cache: {cache_path}")
            with open(cache_path, 'rb') as f:
                documents = pickle.load(f)
            logger.info(f"Loaded {len(documents)} documents from cache")
            return documents
        except Exception as e:
            logger.warning(f"Failed to load cache: {e}, will reprocess")

    # Process repository
    logger.info(f"Processing repository: {repo_url_or_path}")
    documents = self._process_repository(
        repo_url_or_path, type, access_token, is_ollama_embedder,
        excluded_dirs, excluded_files, included_dirs, included_files
    )

    # Save to cache
    try:
        os.makedirs(self.cache_dir, exist_ok=True)
        with open(cache_path, 'wb') as f:
            pickle.dump(documents, f)
        logger.info(f"Saved {len(documents)} documents to cache: {cache_path}")
    except Exception as e:
        logger.warning(f"Failed to save cache: {e}")

    return documents

---

## ⚙️ 配置管理

### 1. 嵌入模型配置

**文件**: `api/config/embedder.json` (第1-20行)

```json
{
  "embedder": {
    "client_class": "OpenAIClient",
    "batch_size": 500,
    "model_kwargs": {
      "model": "text-embedding-3-small",
      "dimensions": 256,
      "encoding_format": "float"
    }
  },
  "retriever": {
    "top_k": 20
  },
  "text_splitter": {
    "split_by": "word",
    "chunk_size": 350,
    "chunk_overlap": 100
  }
}
```

### 2. 生成模型配置

**文件**: `api/config/generator.json` (第1-200行)

#### 多提供商支持
```json
{
  "default_provider": "google",
  "providers": {
    "dashscope": {
      "default_model": "qwen-plus",
      "supportsCustomModel": true,
      "models": {
        "qwen-plus": {
          "temperature": 0.7,
          "top_p": 0.8
        },
        "qwen-turbo": {
          "temperature": 0.7,
          "top_p": 0.8
        },
        "deepseek-r1": {
          "temperature": 0.7,
          "top_p": 0.8
        }
      }
    },
    "google": {
      "default_model": "gemini-2.5-flash",
      "supportsCustomModel": true,
      "models": {
        "gemini-2.5-flash": {
          "temperature": 1.0,
          "top_p": 0.8,
          "top_k": 20
        },
        "gemini-2.5-flash-lite": {
          "temperature": 1.0,
          "top_p": 0.8,
          "top_k": 20
        },
        "gemini-2.5-pro": {
          "temperature": 1.0,
          "top_p": 0.8,
          "top_k": 20
        }
      }
    }
  }
}
```

#### OpenAI配置
```json
{
  "openai": {
    "default_model": "gpt-5-nano",
    "supportsCustomModel": true,
    "models": {
      "gpt-5": {
        "temperature": 1.0
      },
      "gpt-5-nano": {
        "temperature": 1.0
      },
      "gpt-5-mini": {
        "temperature": 1.0
      },
      "gpt-4o": {
        "temperature": 0.7,
        "top_p": 0.8
      },
      "gpt-4.1": {
        "temperature": 0.7,
        "top_p": 0.8
      },
      "o1": {
        "temperature": 0.7,
        "top_p": 0.8
      },
      "o3": {
        "temperature": 1.0
      },
      "o4-mini": {
        "temperature": 1.0
      }
    }
  }
}
```

### 3. 配置加载机制

**文件**: `api/config.py` (第1-100行)

```python
# api/config.py 第1-50行
import json
import os
from typing import Dict, Any

class ConfigManager:
    def __init__(self):
        self.config_dir = os.path.join(os.path.dirname(__file__), 'config')
        self.configs = {}
        self.load_all_configs()

    def load_all_configs(self):
        """Load all configuration files."""
        config_files = [
            'embedder.json',
            'generator.json',
            'lang.json',
            'repo.json'
        ]

        for config_file in config_files:
            config_path = os.path.join(self.config_dir, config_file)
            if os.path.exists(config_path):
                try:
                    with open(config_path, 'r', encoding='utf-8') as f:
                        config_name = config_file.replace('.json', '')
                        self.configs[config_name] = json.load(f)
                        print(f"Loaded config: {config_name}")
                except Exception as e:
                    print(f"Error loading config {config_file}: {e}")

    def get_config(self, config_name: str) -> Dict[str, Any]:
        """Get configuration by name."""
        return self.configs.get(config_name, {})

    def get_embedder_config(self) -> Dict[str, Any]:
        """Get embedder configuration."""
        return self.get_config('embedder')

    def get_generator_config(self) -> Dict[str, Any]:
        """Get generator configuration."""
        return self.get_config('generator')

    def get_language_config(self) -> Dict[str, Any]:
        """Get language configuration."""
        return self.get_config('lang')

# Global config manager instance
config_manager = ConfigManager()
configs = config_manager.configs
```

---

## 🔌 WebSocket实现

### 1. WebSocket聊天处理

**文件**: `api/websocket_wiki.py` (第1-770行)

#### 连接管理
```python
# api/websocket_wiki.py 第50-100行
async def handle_websocket_chat(websocket: WebSocket):
    """Handle WebSocket chat connections."""
    await websocket.accept()
    logger.info("WebSocket connection established")

    try:
        while True:
            # Receive message from client
            data = await websocket.receive_text()
            logger.info(f"Received WebSocket message: {data[:100]}...")

            try:
                # Parse the incoming message
                message = json.loads(data)
                request = ChatRequest(**message)

                # Process the chat request
                await process_chat_request(websocket, request)

            except json.JSONDecodeError as e:
                logger.error(f"Invalid JSON received: {e}")
                await websocket.send_text(json.dumps({
                    "type": "error",
                    "message": "Invalid JSON format"
                }))
            except ValidationError as e:
                logger.error(f"Invalid request format: {e}")
                await websocket.send_text(json.dumps({
                    "type": "error",
                    "message": f"Invalid request format: {str(e)}"
                }))
            except Exception as e:
                logger.error(f"Error processing request: {e}")
                await websocket.send_text(json.dumps({
                    "type": "error",
                    "message": f"Error processing request: {str(e)}"
                }))

    except WebSocketDisconnect:
        logger.info("WebSocket client disconnected")
    except Exception as e:
        logger.error(f"WebSocket error: {e}")
    finally:
        logger.info("WebSocket connection closed")
```

#### 流式响应处理
```python
# api/websocket_wiki.py 第400-450行
async def stream_response(websocket: WebSocket, response_stream, request_id: str):
    """Stream the LLM response to the client."""
    try:
        full_response = ""

        async for chunk in response_stream:
            if hasattr(chunk, 'choices') and chunk.choices:
                delta = chunk.choices[0].delta
                if hasattr(delta, 'content') and delta.content:
                    content = delta.content
                    full_response += content

                    # Send chunk to client
                    await websocket.send_text(json.dumps({
                        "type": "chunk",
                        "request_id": request_id,
                        "content": content,
                        "full_response": full_response
                    }))

        # Send completion signal
        await websocket.send_text(json.dumps({
            "type": "complete",
            "request_id": request_id,
            "full_response": full_response
        }))

    except Exception as e:
        logger.error(f"Error streaming response: {e}")
        await websocket.send_text(json.dumps({
            "type": "error",
            "request_id": request_id,
            "message": f"Error streaming response: {str(e)}"
        }))
```

### 2. 前端WebSocket客户端

**文件**: `src/hooks/useWebSocket.ts` (第1-150行)

```typescript
// src/hooks/useWebSocket.ts 第1-50行
interface WebSocketMessage {
  type: 'chunk' | 'complete' | 'error';
  request_id?: string;
  content?: string;
  full_response?: string;
  message?: string;
}

interface UseWebSocketOptions {
  onMessage?: (message: WebSocketMessage) => void;
  onError?: (error: Event) => void;
  onClose?: (event: CloseEvent) => void;
  onOpen?: (event: Event) => void;
}

export function useWebSocket(url: string, options: UseWebSocketOptions = {}) {
  const [socket, setSocket] = useState<WebSocket | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const ws = new WebSocket(url);

    ws.onopen = (event) => {
      console.log('WebSocket connected');
      setIsConnected(true);
      setError(null);
      options.onOpen?.(event);
    };

    ws.onmessage = (event) => {
      try {
        const message: WebSocketMessage = JSON.parse(event.data);
        options.onMessage?.(message);
      } catch (err) {
        console.error('Failed to parse WebSocket message:', err);
      }
    };

    ws.onerror = (event) => {
      console.error('WebSocket error:', event);
      setError('WebSocket connection error');
      options.onError?.(event);
    };

    ws.onclose = (event) => {
      console.log('WebSocket disconnected');
      setIsConnected(false);
      options.onClose?.(event);
    };

    setSocket(ws);

    return () => {
      ws.close();
    };
  }, [url]);

  const sendMessage = useCallback((message: any) => {
    if (socket && isConnected) {
      socket.send(JSON.stringify(message));
    } else {
      console.error('WebSocket is not connected');
    }
  }, [socket, isConnected]);

  return {
    socket,
    isConnected,
    error,
    sendMessage
  };
}
```

---

## 💾 缓存机制

### 1. Wiki缓存API

**文件**: `src/app/api/wiki/projects/route.ts` (第1-104行)

#### 项目列表获取
```typescript
// src/app/api/wiki/projects/route.ts 第38-73行
export async function GET() {
  try {
    const response = await fetch(PROJECTS_API_ENDPOINT, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        // Add any other headers your Python backend might require, e.g., API keys
      },
      cache: 'no-store', // Ensure fresh data is fetched every time
    });

    if (!response.ok) {
      // Try to parse error from backend, otherwise use status text
      let errorBody = { error: `Failed to fetch from Python backend: ${response.statusText}` };
      try {
        errorBody = await response.json();
      } catch {
        // If parsing JSON fails, errorBody will retain its default value
        // The error from backend is logged in the next line anyway
      }
      console.error(`Error from Python backend (${PROJECTS_API_ENDPOINT}): ${response.status} - ${JSON.stringify(errorBody)}`);
      return NextResponse.json(errorBody, { status: response.status });
    }

    const projects: ApiProcessedProject[] = await response.json();
    return NextResponse.json(projects);

  } catch (error: unknown) {
    console.error(`Network or other error when fetching from ${PROJECTS_API_ENDPOINT}:`, error);
    const message = error instanceof Error ? error.message : 'An unknown error occurred';
    return NextResponse.json(
      { error: `Failed to connect to the Python backend. ${message}` },
      { status: 503 } // Service Unavailable
    );
  }
}
```

#### 缓存删除功能
```typescript
// src/app/api/wiki/projects/route.ts 第75-104行
export async function DELETE(request: Request) {
  try {
    const body: unknown = await request.json();
    if (!isDeleteProjectCachePayload(body)) {
      return NextResponse.json(
        { error: 'Invalid request body: owner, repo, repo_type, and language are required and must be non-empty strings.' },
        { status: 400 }
      );
    }
    const { owner, repo, repo_type, language } = body;
    const params = new URLSearchParams({ owner, repo, repo_type, language });
    const response = await fetch(`${CACHE_API_ENDPOINT}?${params}`, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
    });
    if (!response.ok) {
      let errorBody = { error: response.statusText };
      try {
        errorBody = await response.json();
      } catch {}
      console.error(`Error deleting project cache (${CACHE_API_ENDPOINT}): ${response.status} - ${JSON.stringify(errorBody)}`);
      return NextResponse.json(errorBody, { status: response.status });
    }
    return NextResponse.json({ message: 'Project deleted successfully' });
  } catch (error: unknown) {
    console.error('Error in DELETE /api/wiki/projects:', error);
    const message = error instanceof Error ? error.message : 'An unknown error occurred';
    return NextResponse.json({ error: `Failed to delete project: ${message}` }, { status: 500 });
  }
}
```

### 2. 后端缓存实现

**文件**: `api/tools/database_manager.py` (第300-400行)

```python
# api/tools/database_manager.py 第300-350行
def _generate_cache_key(self, repo_url_or_path: str, type: str,
                       excluded_dirs: List[str] = None, excluded_files: List[str] = None,
                       included_dirs: List[str] = None, included_files: List[str] = None) -> str:
    """Generate a unique cache key for the repository configuration."""
    import hashlib

    # Create a string representation of all parameters
    cache_data = {
        'repo': repo_url_or_path,
        'type': type,
        'excluded_dirs': sorted(excluded_dirs or []),
        'excluded_files': sorted(excluded_files or []),
        'included_dirs': sorted(included_dirs or []),
        'included_files': sorted(included_files or [])
    }

    # Convert to JSON string and hash
    cache_string = json.dumps(cache_data, sort_keys=True)
    cache_hash = hashlib.md5(cache_string.encode()).hexdigest()

    # Include repo name for readability
    repo_name = repo_url_or_path.split('/')[-1].replace('.git', '')
    return f"{repo_name}_{cache_hash}"

def _load_from_cache(self, cache_key: str) -> List[Document]:
    """Load documents from cache if available."""
    cache_path = os.path.join(self.cache_dir, f"{cache_key}.pkl")

    if not os.path.exists(cache_path):
        return None

    try:
        with open(cache_path, 'rb') as f:
            documents = pickle.load(f)
        logger.info(f"Loaded {len(documents)} documents from cache: {cache_path}")
        return documents
    except Exception as e:
        logger.warning(f"Failed to load cache {cache_path}: {e}")
        return None

def _save_to_cache(self, cache_key: str, documents: List[Document]) -> None:
    """Save documents to cache."""
    cache_path = os.path.join(self.cache_dir, f"{cache_key}.pkl")

    try:
        os.makedirs(self.cache_dir, exist_ok=True)
        with open(cache_path, 'wb') as f:
            pickle.dump(documents, f)
        logger.info(f"Saved {len(documents)} documents to cache: {cache_path}")
    except Exception as e:
        logger.warning(f"Failed to save cache {cache_path}: {e}")
```

---

## 📊 总结

DeepWiki的技术架构展现了以下关键特点：

### 🎯 核心优势

1. **模块化设计**: 清晰的前后端分离，组件化的React架构
2. **RAG优化**: 嵌入一致性验证、智能文档分组、上下文格式化
3. **多模式支持**: 简单聊天、深度研究、wiki生成等多种交互模式
4. **配置灵活**: 支持多种LLM提供商和嵌入模型
5. **缓存机制**: 完整的文档处理缓存和wiki生成缓存
6. **实时交互**: WebSocket支持的流式响应和进度更新

### 🛠️ 技术亮点

1. **嵌入一致性验证**: 确保FAISS检索器的稳定性
2. **结构化Prompt**: XML格式的wiki结构定义
3. **优先级生成**: 基于重要性的内容生成策略
4. **多平台支持**: GitHub、GitLab、Bitbucket等
5. **国际化支持**: 完整的多语言框架
6. **错误处理**: 全面的异常处理和用户反馈

这个分析文档详细展示了DeepWiki在RAG系统、Wiki生成、前端架构等方面的技术实现细节，为类似项目的开发提供了宝贵的参考。
```
```
