use crate::common::{Stmt, Expr};

pub struct CodeGenerator {
    pub output: String,
    strings: Vec<(String, String)>,
    variables: Vec<String>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        CodeGenerator {
            output: String::new(),
            strings: Vec::new(),
            variables: Vec::new(),
        }
    }

    pub fn generate(&mut self, ast: &Stmt) {
        // First, we collect all strings and the variables.
        self.collect_strings(ast);

        self.output.push_str("section .data\n");
        
        // FIXME: Integer only variables
        for var in &self.variables {
            self.output.push_str(&format!("{}: dq 0\n", var));
        }
        
        for (_, def) in &self.strings {
            self.output.push_str(def);
        }
        
        // FIXME: These should only be generated if they are used.
        self.output.push_str("fmt_str: db \"%s\", 10, 0\n");
        self.output.push_str("fmt_int: db \"%d\", 10, 0\n");
        
        // FIXME: hardcoded
        self.output.push_str("\nsection .text\n");
        self.output.push_str("default rel\n");
        self.output.push_str("global main\n");
        self.output.push_str("extern printf\n\n");
        
        if let Stmt::Function { name: _, body } = ast {
            self.output.push_str("main:\n");
            // We push rbp to align the stack
            self.output.push_str("push rbp\n");
            for stmt in body {
                self.generate_stmt(stmt);
            }
        }
        // Pop it off again
        self.output.push_str("pop rbp\n");
        // Return code 0
        self.output.push_str("mov rax, 0\n");
        self.output.push_str("ret\n");
    }

    fn collect_strings(&mut self, ast: &Stmt) {
        if let Stmt::Function { name: _, body } = ast {
            for stmt in body {
                self.visit_stmt(stmt);
            }
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VariableDecl { name, .. } => {
                self.variables.push(name.clone());
            }
            Stmt::ExprStmt(expr) => self.visit_expr(expr),
            _ => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call { args, .. } => {
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::StringLiteral(s) => {
                self.define_string(s);
            }
            _ => {}
        }
    }

    fn generate_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            // TODO: The type name should be used.
            Stmt::VariableDecl { name, type_name, value } => {
                self.generate_expr(value);
                self.output.push_str(&format!("mov [{}], rax\n", name));
            }
            Stmt::Assignment { name, value } => {
                self.generate_expr(value);
                self.output.push_str(&format!("mov [{}], rax\n", name));
            }
            Stmt::ExprStmt(expr) => {
                self.generate_expr(expr);
            }
            _ => {}
        }
    }

    fn generate_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call { callee, args } => {
                for arg in args.iter().rev() {
                    self.generate_expr(arg);
                }
                
                match callee.as_str() {
                    "print" => {
                        if let Some(arg) = args.get(0) {
                            match arg {
                                Expr::StringLiteral(_) => {
                                    self.output.push_str("mov rdi, fmt_str\n");
                                }
                                _ => {
                                    // This will also catch booleans, but it is okay,
                                    // as they are internally represented as integers.
                                    // TODO
                                    self.output.push_str("mov rdi, fmt_int\n");
                                }
                            }
                            self.output.push_str("mov rsi, rax\n");
                            self.output.push_str("xor rax, rax\n");
                            self.output.push_str("call printf\n");
                        }
                    }
                    "input" => {
                        // FIXME
                        self.output.push_str("call scanf\n");
                    }
                    _ => {}
                }
            }
            Expr::StringLiteral(s) => {
                let label = self.get_string_label(s);
                self.output.push_str(&format!("mov rax, {}\n", label));
            }
            Expr::IntegerLiteral(n) => {
                self.output.push_str(&format!("mov rax, {}\n", n));
            }
            Expr::BooleanLiteral(b) => {
                let number = if *b { 1 } else { 0 };
                self.output.push_str(&format!("mov rax, {}\n", number));
            }
            Expr::BinaryOperator {operator, left, right } => {
                self.generate_expr(left);
                self.output.push_str("push rax\n");
                self.generate_expr(right);
                self.output.push_str("pop rbx\n");
                println!("Operator: {}", operator);
                match operator.as_str() {
                    "+" => self.output.push_str("add rax, rbx\n"),
                    "-" => self.output.push_str("sub rax, rbx\n"),
                    "*" => self.output.push_str("imul rax, rbx\n"),
                    "/" => self.output.push_str("xor rdx, rdx\nidiv rbx\n"),
                    _ => panic!("Unsupported operator: {}", operator),
                }
            }
            Expr::Variable(name) => {
                self.output.push_str(&format!("mov rax, [{}]\n", name));
            }
        }
    }

    fn define_string(&mut self, s: &str) -> String {
        let label = format!("string_{}", s.replace(' ', "_"));
        let escaped = s.replace('"', r#"\""#);
        let def = format!("{}: db \"{}\", 0\n", label, escaped);
        
        if !self.strings.iter().any(|(l, _)| l == &label) {
            self.strings.push((label.clone(), def));
        }
        
        label
    }

    fn get_string_label(&self, s: &str) -> &str {
        let search = format!("string_{}", s.replace(' ', "_"));
        self.strings.iter()
            .find(|(label, _)| label == &search)
            .map(|(label, _)| label.as_str())
            .expect("String not found in collection")
    }
}
