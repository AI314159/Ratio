use inkwell::AddressSpace;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::PointerValue;
use std::collections::HashMap;

use super::FnSig;
use super::VarKind;
use super::expr::codegen_expr;
use crate::common::Stmt;

pub fn codegen_stmt<'ctx>(
    context: &'ctx Context,
    module: &Module<'ctx>,
    builder: &Builder<'ctx>,
    stmt: &Stmt,
    variables: &mut HashMap<String, VarKind<'ctx>>,
    string_literals: &mut HashMap<String, PointerValue<'ctx>>,
    fmt_int: PointerValue<'ctx>,
    fmt_str: PointerValue<'ctx>,
    function_table: &std::collections::HashMap<String, FnSig<'ctx>>,
) {
    match stmt {
        Stmt::VariableDecl {
            name,
            type_name,
            value,
        } => {
            match type_name.as_str() {
                "int" | "bool" => {
                    let val = codegen_expr(
                        context,
                        module,
                        builder,
                        value,
                        variables,
                        string_literals,
                        fmt_int,
                        fmt_str,
                        function_table,
                    );
                    let ptr = builder.build_alloca(context.i64_type(), name).unwrap();
                    builder
                        .build_store(ptr, val.into_int_value())
                        .expect("store int");
                    variables.insert(name.clone(), VarKind::Int(ptr));
                }
                _ => {
                    // Assume string type.
                    // This shouldn't be reachable because of the parser
                    let val = codegen_expr(
                        context,
                        module,
                        builder,
                        value,
                        variables,
                        string_literals,
                        fmt_int,
                        fmt_str,
                        function_table,
                    );
                    let ptr = builder
                        .build_alloca(context.ptr_type(AddressSpace::default()), name)
                        .unwrap();
                    builder
                        .build_store(ptr, val.into_pointer_value())
                        .expect("store ptr");
                    variables.insert(name.clone(), VarKind::Str(ptr));
                }
            }
        }
        Stmt::Assignment { name, value } => {
            let var_kind = variables.get(name).cloned();
            if let Some(var) = var_kind {
                match var {
                    VarKind::Int(ptr) => {
                        let val = codegen_expr(
                            context,
                            module,
                            builder,
                            value,
                            variables,
                            string_literals,
                            fmt_int,
                            fmt_str,
                            function_table,
                        );
                        builder
                            .build_store(ptr, val.into_int_value())
                            .expect("store int");
                    }
                    VarKind::Str(ptr) => {
                        let val = codegen_expr(
                            context,
                            module,
                            builder,
                            value,
                            variables,
                            string_literals,
                            fmt_int,
                            fmt_str,
                            function_table,
                        );
                        builder
                            .build_store(ptr, val.into_pointer_value())
                            .expect("store ptr");
                    }
                }
            }
        }
        Stmt::ExprStmt(expr) => {
            codegen_expr(
                context,
                module,
                builder,
                expr,
                variables,
                string_literals,
                fmt_int,
                fmt_str,
                function_table,
            );
        }
        Stmt::IfStatement {
            condition,
            body,
            else_body,
        } => {
            let parent = builder.get_insert_block().unwrap().get_parent().unwrap();
            let then_bb = context.append_basic_block(parent, "then");
            let else_bb = context.append_basic_block(parent, "else");
            let merge_bb = context.append_basic_block(parent, "ifcont");

            let cond_val = codegen_expr(
                context,
                module,
                builder,
                condition,
                variables,
                string_literals,
                fmt_int,
                fmt_str,
                function_table,
            );

            let cond_bool = cond_val.into_int_value();
            builder
                .build_conditional_branch(cond_bool, then_bb, else_bb)
                .unwrap();

            // Then
            builder.position_at_end(then_bb);
            for stmt in body {
                codegen_stmt(
                    context,
                    module,
                    builder,
                    stmt,
                    variables,
                    string_literals,
                    fmt_int,
                    fmt_str,
                    function_table,
                );
            }
            builder.build_unconditional_branch(merge_bb).unwrap();

            // Else
            builder.position_at_end(else_bb);
            if let Some(else_body) = else_body {
                for stmt in else_body {
                    codegen_stmt(
                        context,
                        module,
                        builder,
                        stmt,
                        variables,
                        string_literals,
                        fmt_int,
                        fmt_str,
                        function_table,
                    );
                }
            }
            builder.build_unconditional_branch(merge_bb).unwrap();

            builder.position_at_end(merge_bb);
        }
        Stmt::While { condition, body } => {
            let parent = builder.get_insert_block().unwrap().get_parent().unwrap();
            let cond_bb = context.append_basic_block(parent, "while.cond");
            let body_bb = context.append_basic_block(parent, "while.body");
            let after_bb = context.append_basic_block(parent, "while.after");

            builder.build_unconditional_branch(cond_bb).unwrap();
            builder.position_at_end(cond_bb);
            let cond_val = codegen_expr(
                context,
                module,
                builder,
                condition,
                variables,
                string_literals,
                fmt_int,
                fmt_str,
                function_table,
            );

            let cond_bool = cond_val.into_int_value();
            builder
                .build_conditional_branch(cond_bool, body_bb, after_bb)
                .unwrap();

            builder.position_at_end(body_bb);
            for stmt in body {
                codegen_stmt(
                    context,
                    module,
                    builder,
                    stmt,
                    variables,
                    string_literals,
                    fmt_int,
                    fmt_str,
                    function_table,
                );
            }
            builder.build_unconditional_branch(cond_bb).unwrap();

            builder.position_at_end(after_bb);
        }
        _ => {}
    }
}
