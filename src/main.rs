use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

pub struct Compiler {
    pub current_token: String,
    pub output: String,
    pub variables: HashMap<String, String>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            current_token: String::new(),
            output: String::new(),
            variables: HashMap::new(),
        }
    }
}

pub struct LexicalAnalyzer {
    input: Vec<char>,
    position: usize,
    pub tokens: Vec<String>,
}

// lexical analyzer for lolcode
impl LexicalAnalyzer {
    pub fn new(input: &str) -> Self {
        let mut lexer = Self {
            input: input.chars().collect(),
            position: 0,
            tokens: Vec::new(),
        };

        lexer.tokenize();
        lexer
    }

    pub fn get_char(&mut self) -> char {
        if self.position < self.input.len() {
            let c = self.input[self.position];
            self.position += 1;
            c
        } else {
            '\0'
        }
    }

    // set up all valid tokens
    pub fn tokenize(&mut self) {
        let keywords = [
            "#MAEK PARAGRAF",
            "#GIMMEH TITLE",
            "#GIMMEH BOLD",
            "#GIMMEH ITEM",
            "#GIMMEH LINX",
            "#GIMMEH VAR",
            "#MAEK HEAD",
            "#MAEK LIST",
            "#NEWLINE",
            "#IHAZ",
            "#ITIZ",
            "#HAI",
            "#KBYE",
            "#OBTW",
            "#TLDR",
            "#MKAY",
            "#OIC",
        ];

        while self.position < self.input.len() {
            let remaining: String = self.input[self.position..].iter().collect();

            let mut found = false;

            for keyword in keywords {
                if remaining.starts_with(keyword) {
                    self.tokens.push(keyword.to_string());
                    self.position += keyword.chars().count();
                    found = true;
                    break;
                }
            }

            if found {
                continue;
            }

            let c = self.get_char();

            if c.is_whitespace() {
                self.tokens.push("SPACE".to_string());
            } else if c.is_ascii_alphanumeric() {
                self.tokens.push(c.to_string());
            } else if ",.\" :?!%=/".contains(c) {
                self.tokens.push(c.to_string());
            } else {
                eprintln!(
                    // error message for unrecognized character
                    "A lexical error was encountered. '{}' is not a recognized character.",
                    c
                );
                std::process::exit(1);
            }
        }
    }

    pub fn lookup(&self, word: &str) -> bool {
        self.tokens.iter().any(|t| t == word)
    }

    pub fn start(&mut self) -> String {
        if self.tokens.is_empty() {
            // error message if there is no input/input is empty
            eprintln!("A user error was encountered. The input is empty.");
            std::process::exit(1);
        }

        self.tokens.remove(0)
    }

    pub fn next(&mut self) -> String {
        if self.tokens.is_empty() {
            String::new()
        } else {
            self.tokens.remove(0)
        }
    }
}

pub struct SyntaxAnalyzer<'a> {
    lexer: &'a mut LexicalAnalyzer,
    compiler: &'a mut Compiler,
}

// set up syntax analyzer for the 
impl<'a> SyntaxAnalyzer<'a> {
    pub fn new(
        lexer: &'a mut LexicalAnalyzer,
        compiler: &'a mut Compiler,
    ) -> Self {
        Self { lexer, compiler }
    }

    pub fn next_token(&mut self) {
        self.compiler.current_token = self.lexer.next();
    }

    fn check(&self, token: &str) -> bool {
        self.compiler.current_token == token
    }

    fn consume(&mut self, token: &str) {
        if self.check(token) {
            self.next_token();
        } else {
            eprintln!(
                "A syntax error was encountered. '{}' was found when '{}' was expected.",
                self.compiler.current_token,
                token
            );
            std::process::exit(1);
        }
    }

    fn is_text_char(&self) -> bool {
        self.compiler.current_token == "SPACE"
            || self.compiler.current_token.chars().count() == 1
    }

    fn read_text(&mut self) -> String {
        let mut result = String::new();

        while self.is_text_char() {
            if self.check("SPACE") {
                result.push(' ');
            } else {
                result.push_str(&self.compiler.current_token);
            }

            self.next_token();
        }

        result
    }

    fn add_text(&mut self) {
        if self.check("SPACE") {
            self.compiler.output.push(' ');
            self.next_token();
        } else if self.is_text_char() {
            let text = escape_html(&self.compiler.current_token);
            self.compiler.output.push_str(&text);
            self.next_token();
        }
    }

    fn text(&mut self) {
        if !self.is_text_char() {
            eprintln!("A syntax error was encountered. Text was expected.");
            std::process::exit(1);
        }

        while self.is_text_char() {
            self.next_token();
        }
    }

    fn space(&mut self) {
        while self.check("SPACE") {
            self.next_token();
        }
    }

    fn comment(&mut self) {
        self.consume("#OBTW");

        if self.is_text_char() {
            self.text();
        }

        self.consume("#TLDR");
    }

    fn var_def(&mut self) {
        self.consume("#IHAZ");

        let name = self.read_text().trim().to_string();

        self.consume("#ITIZ");

        let value = self.read_text().trim().to_string();

        self.consume("#MKAY");

        self.compiler.variables.insert(name, value);
    }

    fn var_use(&mut self) {
        self.consume("#GIMMEH VAR");

        let name = self.read_text().trim().to_string();

        self.consume("#OIC");

        if let Some(value) = self.compiler.variables.get(&name) {
            let html = escape_html(value);
            self.compiler.output.push_str(&html);
        } else {
            eprintln!("Undefined variable: {}", name);
            std::process::exit(1);
        }
    }

    fn plain_content(&mut self) {
        if self.check("#IHAZ") {
            self.var_def();
        }

        while self.is_text_char() || self.check("#GIMMEH VAR") {
            if self.check("#GIMMEH VAR") {
                self.var_use();
            } else {
                self.add_text();
            }
        }
    }

    fn bold(&mut self) {
        self.consume("#GIMMEH BOLD");

        self.compiler.output.push_str("<strong>");

        self.plain_content();

        self.consume("#OIC");

        self.compiler.output.push_str("</strong>");
    }

    fn link(&mut self) {
        self.consume("#GIMMEH LINX");

        let address = self.read_text().trim().to_string();

        self.consume("#OIC");

        let safe_address = escape_html(&address);

        self.compiler.output.push_str("<a href=\"");
        self.compiler.output.push_str(&safe_address);
        self.compiler.output.push_str("\">");
        self.compiler.output.push_str(&safe_address);
        self.compiler.output.push_str("</a>");
    }

    fn newline(&mut self) {
        self.consume("#NEWLINE");

        self.compiler.output.push_str("<br>\n");
    }

    fn item_item(&mut self) {
        if self.check("#GIMMEH BOLD") {
            self.bold();
        } else if self.check("#NEWLINE") {
            self.newline();
        } else if self.check("#GIMMEH LINX") {
            self.link();
        } else if self.check("#GIMMEH VAR") {
            self.var_use();
        } else if self.check("#OBTW") {
            self.comment();
        } else if self.is_text_char() {
            self.add_text();
        } else {
            eprintln!("A syntax error was encountered inside a list item.");
            std::process::exit(1);
        }
    }

    fn item_content(&mut self) {
        if self.check("#IHAZ") {
            self.var_def();
        }

        while self.is_text_char()
            || self.check("#GIMMEH BOLD")
            || self.check("#NEWLINE")
            || self.check("#GIMMEH LINX")
            || self.check("#GIMMEH VAR")
            || self.check("#OBTW")
        {
            self.item_item();
        }
    }

    fn item(&mut self) {
        self.consume("#GIMMEH ITEM");

        self.compiler.output.push_str("<li>");

        self.item_content();

        self.consume("#OIC");

        self.compiler.output.push_str("</li>\n");
    }

    fn list(&mut self) {
        self.consume("#MAEK LIST");

        self.compiler.output.push_str("<ul>\n");

        self.space();

        if self.check("#IHAZ") {
            self.var_def();
        }

        while self.check("SPACE")
            || self.check("#OBTW")
            || self.check("#GIMMEH ITEM")
        {
            if self.check("SPACE") {
                self.space();
            } else if self.check("#OBTW") {
                self.comment();
            } else {
                self.item();
            }
        }

        self.consume("#MKAY");

        self.compiler.output.push_str("</ul>\n");
    }

    fn par_item(&mut self) {
        if self.check("#GIMMEH BOLD") {
            self.bold();
        } else if self.check("#MAEK LIST") {
            self.list();
        } else if self.check("#NEWLINE") {
            self.newline();
        } else if self.check("#GIMMEH LINX") {
            self.link();
        } else if self.check("#GIMMEH VAR") {
            self.var_use();
        } else if self.check("#OBTW") {
            self.comment();
        } else if self.is_text_char() {
            self.add_text();
        } else {
            eprintln!("A syntax error was encountered inside a paragraph.");
            std::process::exit(1);
        }
    }

    fn paragraph(&mut self) {
        self.consume("#MAEK PARAGRAF");

        self.compiler.output.push_str("<p>");

        if self.check("#IHAZ") {
            self.var_def();
        }

        while self.is_text_char()
            || self.check("#GIMMEH BOLD")
            || self.check("#MAEK LIST")
            || self.check("#NEWLINE")
            || self.check("#GIMMEH LINX")
            || self.check("#GIMMEH VAR")
            || self.check("#OBTW")
        {
            self.par_item();
        }

        self.consume("#MKAY");

        self.compiler.output.push_str("</p>\n");
    }

    fn body_item_nc(&mut self) {
        if self.check("#MAEK PARAGRAF") {
            self.paragraph();
        } else if self.check("#GIMMEH BOLD") {
            self.bold();
        } else if self.check("#MAEK LIST") {
            self.list();
        } else if self.check("#NEWLINE") {
            self.newline();
        } else if self.check("#GIMMEH LINX") {
            self.link();
        } else if self.check("#GIMMEH VAR") {
            self.var_use();
        } else if self.is_text_char() && !self.check("SPACE") {
            self.add_text();
        } else {
            eprintln!(
                "A syntax error was encountered. '{}' is not a valid body item.",
                self.compiler.current_token
            );
            std::process::exit(1);
        }
    }

    fn body_contents(&mut self) {
        while !self.compiler.current_token.is_empty()
            && !self.check("#KBYE")
        {
            if self.check("SPACE") {
                self.compiler.output.push(' ');
                self.next_token();
            } else if self.check("#OBTW") {
                self.comment();
            } else {
                self.body_item_nc();
            }
        }
    }

    fn body(&mut self) {
        self.compiler.output.push_str("<body>\n");

        self.body_contents();

        self.compiler.output.push_str("\n</body>\n");
    }

    fn body_nc(&mut self) {
        self.compiler.output.push_str("<body>\n");

        self.body_item_nc();

        self.body_contents();

        self.compiler.output.push_str("\n</body>\n");
    }

    fn title(&mut self) {
        self.consume("#GIMMEH TITLE");

        self.compiler.output.push_str("<title>");

        self.plain_content();

        self.consume("#OIC");

        self.compiler.output.push_str("</title>\n");
    }

    fn head_start(&mut self) {
        while self.check("SPACE") {
            self.space();
        }

        if self.check("#IHAZ") {
            self.var_def();
            self.head_rest();
        } else if self.check("#OBTW") {
            self.comment();
            self.head_rest();
        } else if self.check("#GIMMEH TITLE") {
            self.title();
            self.head_end();
        }
    }

    fn head_rest(&mut self) {
        while self.check("SPACE") {
            self.space();
        }

        if self.check("#OBTW") {
            self.comment();
            self.head_rest();
        } else if self.check("#GIMMEH TITLE") {
            self.title();
            self.head_end();
        }
    }

    fn head_end(&mut self) {
        self.space();
        if self.check("#OBTW") {
            self.comment();
            self.head_end();
        }
    }

    fn head(&mut self) {
        self.consume("#MAEK HEAD");

        self.compiler.output.push_str("<head>\n");

        self.head_start();

        self.consume("#MKAY");

        self.compiler.output.push_str("</head>\n");
    }

    fn lolcode_rest(&mut self) {
        while self.check("SPACE") {
            self.space();
        }

        while self.check("#OBTW") {
            self.comment();
            self.space();
        }

        if self.check("#MAEK HEAD") {
            self.head();
            self.body();
        } else {
            self.body_nc();
        }
    }

    fn lolcode_content(&mut self) {
        while self.check("SPACE") {
            self.space();
        }

        if self.check("#IHAZ") {
            self.var_def();
            self.lolcode_rest();
        } else if self.check("#OBTW") {
            self.comment();
            self.lolcode_rest();
        } else if self.check("#MAEK HEAD") {
            self.head();
            self.body();
        } else {
            self.body_nc();
        }
    }

    pub fn lolcode(&mut self) {
        self.consume("#HAI");

        self.compiler.output.push_str(
            "<!DOCTYPE html>\n<html>\n",
        );

        self.lolcode_content();

        self.consume("#KBYE");

        while self.check("SPACE") {
            self.space();
        }

        if !self.compiler.current_token.is_empty() {
            eprintln!(
                "A syntax error was encountered. Additional tokens found after #KBYE."
            );
            std::process::exit(1);
        }

        self.compiler.output.push_str("</html>\n");
    }
}

fn escape_html(text: &str) -> String {
    text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#39;")
}

fn open_in_browser(path: &Path) {
    #[cfg(target_os = "windows")]
    let result = Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(path)
        .spawn();

    #[cfg(target_os = "macos")]
    let result = Command::new("open")
        .arg(path)
        .spawn();

    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open")
        .arg(path)
        .spawn();

    if let Err(error) = result {
        eprintln!("Could not open the browser: {}", error);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: lolcompiler <input.lol>");
        return;
    }

    let input_path = Path::new(&args[1]);

    if input_path.extension().and_then(|ext| ext.to_str()) != Some("lol") {
        eprintln!("The input file must have a .lol extension.");
        return;
    }

    let sentence = match std::fs::read_to_string(input_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("Could not read the input file: {}", error);
            return;
        }
    };

    let output_path = input_path.with_extension("html");

    let mut compiler = Compiler::new();

    let mut lexer = LexicalAnalyzer::new(&sentence);

    compiler.current_token = lexer.start();

    let mut parser = SyntaxAnalyzer::new(&mut lexer, &mut compiler);

    parser.lolcode();

    drop(parser);
    let html = compiler.output.clone();

    if let Err(error) = std::fs::write(&output_path, html) {
        eprintln!("Could not write the HTML file: {}", error);
        return;
    }

    println!("HTML file created: {}", output_path.display());

    open_in_browser(&output_path);
}