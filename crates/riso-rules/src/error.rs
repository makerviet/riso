use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuleError {
    pub code: &'static str,
    pub path: String,
    pub line: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScoreError {
    pub code: &'static str,
    pub path: String,
    pub message: String,
}

impl RuleError {
    pub fn new(code: &'static str, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            path: path.into(),
            line: None,
            message: message.into(),
        }
    }

    pub fn located(mut self, src: &str) -> Self {
        if self.line.is_none() {
            self.line = line_of(src, &self.path);
        }
        self
    }
}

impl ScoreError {
    pub fn new(code: &'static str, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for RuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(
                f,
                "[{}] {} (dòng {}): {}",
                self.code, self.path, line, self.message
            ),
            None => write!(f, "[{}] {}: {}", self.code, self.path, self.message),
        }
    }
}

impl std::fmt::Display for ScoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.code, self.path, self.message)
    }
}

/// Tìm dòng (1-based) của khoá theo đường dẫn chấm trong YAML dạng khối.
/// Chỉ dùng để chỉ chỗ cho người viết luật; không tìm thấy thì trả `None`.
pub fn line_of(src: &str, path: &str) -> Option<usize> {
    let lines: Vec<&str> = src.lines().collect();
    let mut from = 0;
    let mut min_indent = 0;
    let mut found = None;
    for segment in path.split('.') {
        let mut hit = None;
        for (index, line) in lines.iter().enumerate().skip(from) {
            let indent = line.len() - line.trim_start().len();
            let trimmed = line.trim_start();
            if found.is_some()
                && !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && indent < min_indent
            {
                break;
            }
            if indent >= min_indent
                && (trimmed.starts_with(&format!("{segment}:"))
                    || trimmed.starts_with(&format!("\"{segment}\":")))
            {
                hit = Some((index, indent));
                break;
            }
        }
        let (index, indent) = hit?;
        found = Some(index + 1);
        from = index + 1;
        min_indent = indent + 1;
    }
    found
}
