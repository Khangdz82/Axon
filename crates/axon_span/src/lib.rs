//! axon_span: Source code location, spans, and error diagnostics

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FileId(pub u32);

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FileId({})", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ByteSpan {
    pub start: u32,
    pub end: u32,
    pub file_id: FileId,
}

impl ByteSpan {
    pub const DUMMY: Self = ByteSpan {
        start: 0,
        end: 0,
        file_id: FileId(0),
    };

    pub fn new(start: u32, end: u32, file_id: FileId) -> Self {
        Self { start, end, file_id }
    }

    pub fn merge(self, other: ByteSpan) -> Self {
        if self.file_id != other.file_id {
            return self;
        }
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            file_id: self.file_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: FileId,
    pub name: String,
    pub src: String,
    line_starts: Vec<u32>,
}

impl SourceFile {
    pub fn new(id: FileId, name: String, src: String) -> Self {
        let mut line_starts = vec![0];
        for (i, byte) in src.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        Self {
            id,
            name,
            src,
            line_starts,
        }
    }

    pub fn line_col(&self, offset: u32) -> (usize, usize) {
        let offset = offset.min(self.src.len() as u32);
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };
        let line = line_idx + 1;
        let col = (offset - self.line_starts[line_idx] + 1) as usize;
        (line, col)
    }

    pub fn get_line(&self, line: usize) -> Option<&str> {
        if line == 0 || line > self.line_starts.len() {
            return None;
        }
        let start = self.line_starts[line - 1] as usize;
        let end = if line < self.line_starts.len() {
            self.line_starts[line] as usize
        } else {
            self.src.len()
        };
        Some(self.src[start..end].trim_end_matches(['\r', '\n']))
    }
}

#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, name: String, src: String) -> FileId {
        let id = FileId(self.files.len() as u32);
        self.files.push(SourceFile::new(id, name, src));
        id
    }

    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }

    pub fn format_error(&self, span: ByteSpan, msg: &str) -> String {
        let Some(file) = self.get(span.file_id) else {
            return format!("Error [span {:?}]: {}", span, msg);
        };
        let (line, col) = file.line_col(span.start);
        let line_text = file.get_line(line).unwrap_or("");
        let indent = " ".repeat(col.saturating_sub(1));
        format!(
            "error: {}\n  --> {}:{}:{}\n   |\n{:4} | {}\n   | {}^\n",
            msg, file.name, line, col, line, line_text, indent
        )
    }
}
