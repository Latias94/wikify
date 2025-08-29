/**
 * Markdown渲染测试
 * 验证Streamdown集成和各种Markdown功能
 */

import { render, screen } from '@testing-library/react';
import { StreamingContent } from '../StreamingContent';

// Mock Streamdown
jest.mock('streamdown', () => ({
  Streamdown: ({ children, className }: any) => (
    <div className={className} data-testid="streamdown">
      {children}
    </div>
  ),
}));

describe('Markdown Rendering', () => {
  it('should render basic text', () => {
    render(<StreamingContent content="Hello, world!" />);
    expect(screen.getByText('Hello, world!')).toBeInTheDocument();
  });

  it('should use Streamdown component', () => {
    render(<StreamingContent content="Test content" />);
    expect(screen.getByTestId('streamdown')).toBeInTheDocument();
  });

  it('should apply custom className', () => {
    render(<StreamingContent content="Test" className="custom-class" />);
    const streamdown = screen.getByTestId('streamdown');
    expect(streamdown).toHaveClass('custom-class');
  });

  it('should handle empty content', () => {
    render(<StreamingContent content="" />);
    const streamdown = screen.getByTestId('streamdown');
    expect(streamdown).toBeInTheDocument();
  });

  it('should handle markdown content', () => {
    const markdownContent = `
# Heading 1
## Heading 2

This is a paragraph with **bold** and *italic* text.

\`\`\`javascript
console.log('Hello, world!');
\`\`\`

- List item 1
- List item 2

| Column 1 | Column 2 |
|----------|----------|
| Cell 1   | Cell 2   |
`;

    render(<StreamingContent content={markdownContent} />);
    expect(screen.getByTestId('streamdown')).toBeInTheDocument();
  });

  it('should handle code blocks', () => {
    const codeContent = `
\`\`\`typescript
interface User {
  id: string;
  name: string;
  email: string;
}

const user: User = {
  id: '1',
  name: 'John Doe',
  email: 'john@example.com'
};
\`\`\`
`;

    render(<StreamingContent content={codeContent} />);
    expect(screen.getByTestId('streamdown')).toBeInTheDocument();
  });

  it('should handle mermaid diagrams', () => {
    const mermaidContent = `
\`\`\`mermaid
graph TD
    A[Start] --> B{Is it?}
    B -->|Yes| C[OK]
    C --> D[Rethink]
    D --> B
    B ---->|No| E[End]
\`\`\`
`;

    render(<StreamingContent content={mermaidContent} />);
    expect(screen.getByTestId('streamdown')).toBeInTheDocument();
  });

  it('should handle mathematical expressions', () => {
    const mathContent = `
Here's an inline math expression: $E = mc^2$

And here's a block equation:

$$
\\int_{-\\infty}^{\\infty} e^{-x^2} dx = \\sqrt{\\pi}
$$
`;

    render(<StreamingContent content={mathContent} />);
    expect(screen.getByTestId('streamdown')).toBeInTheDocument();
  });

  it('should handle tables', () => {
    const tableContent = `
| Feature | Vercel AI Chatbot | Our Implementation |
|---------|-------------------|-------------------|
| Markdown | ✅ Streamdown | ✅ Streamdown |
| Code Highlighting | ✅ Shiki | ✅ Shiki |
| Math Support | ✅ KaTeX | ✅ KaTeX |
| Mermaid Diagrams | ✅ | ✅ |
`;

    render(<StreamingContent content={tableContent} />);
    expect(screen.getByTestId('streamdown')).toBeInTheDocument();
  });

  it('should handle links and images', () => {
    const linkContent = `
Check out [Vercel AI Chatbot](https://github.com/vercel/ai-chatbot) for reference.

![Example Image](https://example.com/image.png)
`;

    render(<StreamingContent content={linkContent} />);
    expect(screen.getByTestId('streamdown')).toBeInTheDocument();
  });

  it('should handle blockquotes', () => {
    const quoteContent = `
> This is a blockquote.
> It can span multiple lines.
>
> And have multiple paragraphs.
`;

    render(<StreamingContent content={quoteContent} />);
    expect(screen.getByTestId('streamdown')).toBeInTheDocument();
  });
});
