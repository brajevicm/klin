//! The bounded shell reader the guard proves writes with. It honours quotes, the list
//! operators, redirects, comments and heredoc bodies, and it flags the shell it does not read
//! rather than guess at it. It never runs a shell. ADR 0033, ADR 0068.

/// The operator that ends a segment, which says whether the next one runs in the same shell
/// and only after this one succeeded.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Op {
    #[default]
    List,
    And,
    Or,
    Pipe,
    Background,
}

/// One simple command: its words with the quotes taken out, the files its `>` redirects name,
/// the body of a heredoc it reads, and the operator after it.
#[derive(Default, Debug)]
pub struct Segment {
    pub words: Vec<String>,
    pub targets: Vec<String>,
    pub body: Option<String>,
    pub after: Op,
}

pub struct Lexed {
    pub segments: Vec<Segment>,
    /// The command holds shell klin does not read: `$(`, `${`, a backtick, a backslash, a
    /// here-string, a process substitution or a heredoc left open.
    pub unreadable: bool,
    /// The command holds a subshell, a group or a function, so a `cd` in it may not reach the
    /// next command.
    pub grouped: bool,
}

/// The command read into segments, and `None` when a quote is left open, because then the
/// command does not say where its arguments end.
pub fn lexed(command: &str) -> Option<Lexed> {
    Lexer::new(command).run()
}

enum Slot {
    Word,
    Target,
    Dropped,
    Delimiter { strip: bool },
}

struct Heredoc {
    delimiter: String,
    strip: bool,
    segment: usize,
}

struct Lexer<'a> {
    text: &'a str,
    at: usize,
    quotes: Quotes,
    word: String,
    started: bool,
    next: Slot,
    current: Segment,
    heredocs: Vec<Heredoc>,
    done: Vec<Segment>,
    unreadable: bool,
    grouped: bool,
}

impl<'a> Lexer<'a> {
    fn new(text: &'a str) -> Lexer<'a> {
        Lexer {
            text,
            at: 0,
            quotes: Quotes::default(),
            word: String::new(),
            started: false,
            next: Slot::Word,
            current: Segment::default(),
            heredocs: Vec::new(),
            done: Vec::new(),
            unreadable: false,
            grouped: false,
        }
    }

    fn run(mut self) -> Option<Lexed> {
        while let Some(c) = self.peek() {
            self.at += c.len_utf8();
            self.step(c);
        }
        if self.quotes.any() {
            return None;
        }
        self.finish_word();
        self.unreadable |= !self.heredocs.is_empty();
        self.end_segment(Op::List);
        Some(Lexed {
            segments: self.done,
            unreadable: self.unreadable,
            grouped: self.grouped,
        })
    }

    fn step(&mut self, c: char) {
        let next = self.peek();
        self.unreadable |= matches!(c, '`' | '\\') || (c == '$' && matches!(next, Some('(' | '{')));
        if self.quotes.toggled_by(c) {
            self.started = true;
        } else if self.quotes.any() || !self.operator(c) {
            self.word.push(c);
            self.started = true;
        }
    }

    fn operator(&mut self, c: char) -> bool {
        match c {
            ' ' | '\t' => self.finish_word(),
            '\n' => self.newline(),
            ';' | '&' | '|' => self.separator(c),
            '>' | '<' => self.redirect(c),
            '#' if !self.started => self.comment(),
            '(' | ')' | '{' | '}' => return self.group(c),
            _ => return false,
        }
        true
    }

    fn separator(&mut self, c: char) {
        self.finish_word();
        let op = match c {
            ';' => Op::List,
            '&' if self.eat('&') => Op::And,
            '&' if self.peek() == Some('>') => return,
            '&' => Op::Background,
            _ if self.eat('|') => Op::Or,
            _ => {
                self.eat('&');
                Op::Pipe
            }
        };
        self.end_segment(op);
    }

    /// A redirect, whose file descriptor number in front is no argument of the command.
    fn redirect(&mut self, c: char) {
        if self.word.bytes().all(|b| b.is_ascii_digit()) {
            self.word.clear();
            self.started = false;
        }
        self.finish_word();
        self.next = match c {
            '>' => self.output(),
            _ => self.input(),
        };
    }

    fn output(&mut self) -> Slot {
        let _ = self.eat('>') || self.eat('|') || self.eat('&');
        self.unreadable |= self.peek() == Some('(');
        Slot::Target
    }

    fn input(&mut self) -> Slot {
        if self.text[self.at..].starts_with("<<") {
            self.unreadable = true;
            return Slot::Dropped;
        }
        if self.eat('<') {
            return Slot::Delimiter {
                strip: self.eat('-'),
            };
        }
        self.unreadable |= self.peek() == Some('(');
        if self.eat('>') {
            return Slot::Target;
        }
        self.eat('&');
        Slot::Dropped
    }

    fn comment(&mut self) {
        let rest = &self.text[self.at..];
        self.at += rest.find('\n').unwrap_or(rest.len());
    }

    fn group(&mut self, c: char) -> bool {
        self.grouped = true;
        let substitution = c == '(' && self.word.ends_with('$');
        let operator = matches!(c, '(' | ')') && !substitution;
        if operator {
            self.finish_word();
        }
        operator
    }

    fn newline(&mut self) {
        self.finish_word();
        self.end_segment(Op::List);
        for heredoc in std::mem::take(&mut self.heredocs) {
            let body = self.body(&heredoc);
            self.unreadable |= body.is_none();
            if let Some(segment) = self.done.get_mut(heredoc.segment) {
                segment.body = body;
            }
        }
    }

    /// The lines up to the heredoc's delimiter, and `None` when the command ends first.
    fn body(&mut self, heredoc: &Heredoc) -> Option<String> {
        let mut body = String::new();
        while self.at < self.text.len() {
            let rest = &self.text[self.at..];
            let line = rest.split('\n').next().unwrap_or(rest);
            self.at += (line.len() + 1).min(rest.len());
            let line = match heredoc.strip {
                true => line.trim_start_matches('\t'),
                false => line,
            };
            if line == heredoc.delimiter {
                return Some(body);
            }
            body.push_str(line);
            body.push('\n');
        }
        None
    }

    fn finish_word(&mut self) {
        if !self.started {
            return;
        }
        self.started = false;
        let word = std::mem::take(&mut self.word);
        match std::mem::replace(&mut self.next, Slot::Word) {
            Slot::Word => self.current.words.push(word),
            Slot::Target => self.current.targets.push(word),
            Slot::Dropped => {}
            Slot::Delimiter { strip } => self.heredocs.push(Heredoc {
                delimiter: word,
                strip,
                segment: self.done.len(),
            }),
        }
    }

    /// Ends the segment under the operator, and drops an empty one, so the operator before a
    /// blank line still reaches the command after it.
    fn end_segment(&mut self, after: Op) {
        let index = self.done.len();
        let empty = self.current.words.is_empty()
            && self.current.targets.is_empty()
            && !self.heredocs.iter().any(|heredoc| heredoc.segment == index);
        if empty {
            return;
        }
        let mut segment = std::mem::take(&mut self.current);
        segment.after = after;
        self.done.push(segment);
    }

    fn peek(&self) -> Option<char> {
        self.text[self.at..].chars().next()
    }

    fn eat(&mut self, c: char) -> bool {
        let hit = self.peek() == Some(c);
        if hit {
            self.at += c.len_utf8();
        }
        hit
    }
}

/// A single quote makes everything literal, and a double quote makes a separator literal.
#[derive(Default)]
struct Quotes {
    single: bool,
    double: bool,
}

impl Quotes {
    fn toggled_by(&mut self, c: char) -> bool {
        match c {
            '\'' if !self.double => self.single = !self.single,
            '"' if !self.single => self.double = !self.double,
            _ => return false,
        }
        true
    }

    fn any(&self) -> bool {
        self.single || self.double
    }
}
