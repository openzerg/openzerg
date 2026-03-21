use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::error::{Error, Result};
use super::traits::{Tool, ToolContext, ToolResult};
use super::schema::generate_schema;
use super::truncation::{truncate_output, MAX_BYTES};

const DESCRIPTION: &str = include_str!("prompts/webfetch.md");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WebFetchParams {
    #[schemars(description = "The URL to fetch")]
    pub url: String,
    #[schemars(description = "The format to return content in: markdown, text, or html")]
    pub format: Option<String>,
}

pub struct WebFetchTool;

impl WebFetchTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn id(&self) -> &str { "webfetch" }
    
    fn description(&self) -> &str {
        DESCRIPTION
    }
    
    fn parameters_schema(&self) -> Value {
        generate_schema::<WebFetchParams>()
    }
    
    async fn execute(&self, args: Value, _ctx: ToolContext) -> Result<ToolResult> {
        let params: WebFetchParams = serde_json::from_value(args)
            .map_err(|e| Error::Tool(format!("Invalid parameters: {}", e)))?;
        
        let format = params.format.unwrap_or_else(|| "markdown".to_string());
        
        if !params.url.starts_with("http://") && !params.url.starts_with("https://") {
            return Err(Error::Tool("URL must start with http:// or https://".into()));
        }
        
        let response = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Tool(format!("Failed to build client: {}", e)))?
            .get(&params.url)
            .send()
            .await
            .map_err(|e| Error::Tool(format!("Failed to fetch URL: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(Error::Tool(format!("HTTP error: {}", response.status())));
        }
        
        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/plain")
            .to_string();
        
        let body = response.text().await
            .map_err(|e| Error::Tool(format!("Failed to read response: {}", e)))?;
        
        let output = match format.as_str() {
            "html" => body,
            "text" => html_to_text(&body),
            "markdown" | _ => html_to_markdown(&body),
        };
        
        let (output, truncated) = truncate_output(&output, MAX_BYTES);
        
        Ok(ToolResult {
            title: params.url.clone(),
            output,
            metadata: serde_json::json!({
                "url": params.url,
                "format": format,
                "content_type": content_type,
            }),
            attachments: vec![],
            truncated,
        })
    }
}

fn html_to_text(html: &str) -> String {
    let re_tags = regex::Regex::new(r"<[^>]+>").unwrap();
    let text = re_tags.replace_all(html, " ");
    let re_whitespace = regex::Regex::new(r"\s+").unwrap();
    re_whitespace.replace_all(&text, " ").trim().to_string()
}

fn html_to_markdown(html: &str) -> String {
    let mut md = html.to_string();
    
    let re_h1 = regex::Regex::new(r"<h1[^>]*>(.*?)</h1>").unwrap();
    let re_h2 = regex::Regex::new(r"<h2[^>]*>(.*?)</h2>").unwrap();
    let re_h3 = regex::Regex::new(r"<h3[^>]*>(.*?)</h3>").unwrap();
    let re_p = regex::Regex::new(r"<p[^>]*>(.*?)</p>").unwrap();
    let re_a = regex::Regex::new(r#"<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#).unwrap();
    let re_strong = regex::Regex::new(r"<strong[^>]*>(.*?)</strong>").unwrap();
    let re_em = regex::Regex::new(r"<em[^>]*>(.*?)</em>").unwrap();
    let re_code = regex::Regex::new(r"<code[^>]*>(.*?)</code>").unwrap();
    let re_pre = regex::Regex::new(r"<pre[^>]*>(.*?)</pre>").unwrap();
    let re_li = regex::Regex::new(r"<li[^>]*>(.*?)</li>").unwrap();
    let re_br = regex::Regex::new(r"<br\s*/?>").unwrap();
    
    md = re_h1.replace_all(&md, "\n# $1\n\n").to_string();
    md = re_h2.replace_all(&md, "\n## $1\n\n").to_string();
    md = re_h3.replace_all(&md, "\n### $1\n\n").to_string();
    md = re_p.replace_all(&md, "\n$1\n").to_string();
    md = re_a.replace_all(&md, "[$2]($1)").to_string();
    md = re_strong.replace_all(&md, "**$1**").to_string();
    md = re_em.replace_all(&md, "*$1*").to_string();
    md = re_code.replace_all(&md, "`$1`").to_string();
    md = re_pre.replace_all(&md, "\n```\n$1\n```\n").to_string();
    md = re_li.replace_all(&md, "- $1\n").to_string();
    md = re_br.replace_all(&md, "\n").to_string();
    
    let re_tags = regex::Regex::new(r"<[^>]+>").unwrap();
    md = re_tags.replace_all(&md, "").to_string();
    
    let re_whitespace = regex::Regex::new(r"\n{3,}").unwrap();
    md = re_whitespace.replace_all(&md, "\n\n").to_string();
    
    md.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webfetch_tool_id() {
        let tool = WebFetchTool::new();
        assert_eq!(tool.id(), "webfetch");
    }

    #[test]
    fn test_webfetch_tool_description() {
        let tool = WebFetchTool::new();
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_webfetch_params_deserialize() {
        let json = serde_json::json!({
            "url": "https://example.com"
        });
        let params: WebFetchParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.url, "https://example.com");
        assert!(params.format.is_none());
    }

    #[test]
    fn test_webfetch_params_deserialize_with_format() {
        let json = serde_json::json!({
            "url": "https://example.com",
            "format": "text"
        });
        let params: WebFetchParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.url, "https://example.com");
        assert_eq!(params.format, Some("text".to_string()));
    }

    #[test]
    fn test_html_to_text_simple() {
        let html = "<p>Hello World</p>";
        let text = html_to_text(html);
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_html_to_text_with_tags() {
        let html = "<div><span>Hello</span> <span>World</span></div>";
        let text = html_to_text(html);
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_html_to_text_whitespace() {
        let html = "<p>  Hello   World  </p>";
        let text = html_to_text(html);
        assert_eq!(text, "Hello World");
    }

    #[test]
    fn test_html_to_markdown_h1() {
        let html = "<h1>Title</h1>";
        let md = html_to_markdown(html);
        assert!(md.contains("# Title"));
    }

    #[test]
    fn test_html_to_markdown_h2() {
        let html = "<h2>Subtitle</h2>";
        let md = html_to_markdown(html);
        assert!(md.contains("## Subtitle"));
    }

    #[test]
    fn test_html_to_markdown_h3() {
        let html = "<h3>Section</h3>";
        let md = html_to_markdown(html);
        assert!(md.contains("### Section"));
    }

    #[test]
    fn test_html_to_markdown_paragraph() {
        let html = "<p>This is a paragraph.</p>";
        let md = html_to_markdown(html);
        assert!(md.contains("This is a paragraph."));
    }

    #[test]
    fn test_html_to_markdown_link() {
        let html = r#"<a href="https://example.com">Example</a>"#;
        let md = html_to_markdown(html);
        assert!(md.contains("[Example](https://example.com)"));
    }

    #[test]
    fn test_html_to_markdown_strong() {
        let html = "<strong>bold</strong>";
        let md = html_to_markdown(html);
        assert!(md.contains("**bold**"));
    }

    #[test]
    fn test_html_to_markdown_em() {
        let html = "<em>italic</em>";
        let md = html_to_markdown(html);
        assert!(md.contains("*italic*"));
    }

    #[test]
    fn test_html_to_markdown_code() {
        let html = "<code>inline code</code>";
        let md = html_to_markdown(html);
        assert!(md.contains("`inline code`"));
    }

    #[test]
    fn test_html_to_markdown_pre() {
        let html = "<pre>code block</pre>";
        let md = html_to_markdown(html);
        assert!(md.contains("```"));
        assert!(md.contains("code block"));
    }

    #[test]
    fn test_html_to_markdown_li() {
        let html = "<li>item</li>";
        let md = html_to_markdown(html);
        assert!(md.contains("- item"));
    }

    #[test]
    fn test_html_to_markdown_br() {
        let html = "line1<br>line2";
        let md = html_to_markdown(html);
        assert!(md.contains("line1"));
        assert!(md.contains("line2"));
    }

    #[test]
    fn test_html_to_markdown_complex() {
        let html = r#"<h1>Title</h1><p>Text with <strong>bold</strong> and <a href="url">link</a>.</p>"#;
        let md = html_to_markdown(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("**bold**"));
        assert!(md.contains("[link](url)"));
    }
}