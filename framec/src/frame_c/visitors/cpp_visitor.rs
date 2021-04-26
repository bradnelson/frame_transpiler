#![allow(non_snake_case)]

use super::super::ast::*;
use super::super::symbol_table::*;
use super::super::symbol_table::SymbolType::*;
use super::super::visitors::*;
use super::super::scanner::{Token,TokenType};
use downcast_rs::__std::env::var;
use crate::frame_c::ast::MessageType::CustomMessage;


pub struct CppVisitor {
    compiler_version:String,
    pub code:String,
    pub dent:usize,
    pub current_state_name_opt:Option<String>,
    current_event_ret_type:String,
    arcanium:Arcanum,
    symbol_config:SymbolConfig,
    comments:Vec<Token>,
    current_comment_idx:usize,
    first_event_handler:bool,
    system_name:String,
    first_state_name:String,
    warnings:Vec<String>,
    has_states:bool,
    errors:Vec<String>,
    visiting_call_chain_literal_variable:bool,
    generate_exit_args:bool,
    generate_state_context:bool,
    generate_state_stack:bool,
    generate_change_state:bool,
    generate_transition_state:bool,
}

impl CppVisitor {

    //* --------------------------------------------------------------------- *//

    pub fn new(  arcanium:Arcanum
               , generate_exit_args:bool
               , generate_state_context:bool
               , generate_state_stack:bool
               , generate_change_state:bool
               , generate_transition_state:bool
               , compiler_version:&str
               , comments:Vec<Token>) -> CppVisitor {

        // These closures are needed to do the same actions as add_code() and newline()
        // when inside a borrowed self reference as they modify self.
        // let mut add_code_cl = |target:&mut String, s:&String| target.push_str(s);
        // let mut newline_cl = |target:&mut String, s:&String, d:usize| {
        //     target.push_str(&*format!("\n{}",(0..d).map(|_| "\t").collect::<String>()));
        // };

        CppVisitor {
            compiler_version:compiler_version.to_string(),
            code:String::from(""),
            dent:0,
            current_state_name_opt:None,
            current_event_ret_type:String::new(),
            arcanium,
            symbol_config:SymbolConfig::new(),
            comments,
            current_comment_idx:0,
            first_event_handler:true,
            system_name:String::new(),
            first_state_name:String::new(),
            has_states:false,
            warnings:Vec::new(),
            visiting_call_chain_literal_variable:false,
            errors:Vec::new(),
            generate_exit_args,
            generate_state_context,
            generate_state_stack,
            generate_change_state,
            generate_transition_state,
        }
    }

    //* --------------------------------------------------------------------- *//

    fn get_variable_type(&self,symbol_type:&SymbolType) -> String {
        let var_type = match &*symbol_type {
            DomainVariableSymbolT { domain_variable_symbol_rcref } => {
                match &domain_variable_symbol_rcref.borrow().var_type {
                    Some(x) => x.clone(),
                    None => String::from("<?>"),
                }
            },
            StateParamSymbolT { state_param_symbol_rcref } => {
                match &state_param_symbol_rcref.borrow().param_type {
                    Some(x) => x.clone(),
                    None => String::from("<?>"),
                }
            },
            StateVariableSymbolT { state_variable_symbol_rcref } => {
                match &state_variable_symbol_rcref.borrow().var_type {
                    Some(x) => x.clone(),
                    None => String::from("<?>"),
                }                    },
            EventHandlerParamSymbolT { event_handler_param_symbol_rcref } => {
                match &event_handler_param_symbol_rcref.borrow().param_type {
                    Some(x) => x.clone(),
                    None => String::from("<?>"),
                }
            },
            EventHandlerVariableSymbolT { event_handler_variable_symbol_rcref } => {
                match &event_handler_variable_symbol_rcref.borrow().var_type {
                    Some(x) => x.clone(),
                    None => String::from("<?>"),
                }
            },

            _ => panic!("TODO"),
        };

        return var_type;
    }

    //* --------------------------------------------------------------------- *//

    fn format_variable_expr(&self, variable_node:&VariableNode) -> String {
        let mut code = String::new();

        match variable_node.scope {
            IdentifierDeclScope::DomainBlock => {
                code.push_str(&format!("this->{}",variable_node.id_node.name.lexeme));
            },
            IdentifierDeclScope::StateParam => {
                let var_node = variable_node;
                let var_symbol_rcref_opt = &var_node.symbol_type_rcref_opt;
                let var_symbol_rcref = var_symbol_rcref_opt.as_ref().unwrap();
                let var_symbol = var_symbol_rcref.borrow();
                let var_type = self.get_variable_type(&*var_symbol);

                if self.visiting_call_chain_literal_variable {
                    code.push_str("(");
                }
                code.push_str(&format!("*({})*) _pStateContext_->getStateArg(\"{}\")"
                                       ,var_type
                                       ,variable_node.id_node.name.lexeme));
                if self.visiting_call_chain_literal_variable {
                    code.push_str(")");
                }
            },
            IdentifierDeclScope::StateVar => {
                let var_node = variable_node;
                let var_symbol_rcref_opt = &var_node.symbol_type_rcref_opt;
                let var_symbol_rcref = var_symbol_rcref_opt.as_ref().unwrap();
                let var_symbol = var_symbol_rcref.borrow();
                let var_type = self.get_variable_type(&*var_symbol);

                if self.visiting_call_chain_literal_variable {
                    code.push_str("(");
                }
                code.push_str(&format!("*({}*) _pStateContext_->getStateVar(\"{}\")"
                                       ,var_type
                                       ,variable_node.id_node.name.lexeme));
                if self.visiting_call_chain_literal_variable {
                    code.push_str(")");
                }
            }
            IdentifierDeclScope::EventHandlerParam => {
                let var_node = variable_node;
                let var_symbol_rcref_opt = &var_node.symbol_type_rcref_opt;
                let var_symbol_rcref = var_symbol_rcref_opt.as_ref().unwrap();
                let var_symbol = var_symbol_rcref.borrow();
                let var_type = self.get_variable_type(&*var_symbol);

                if self.visiting_call_chain_literal_variable {
                    code.push_str("(");
                }
                code.push_str(&format!("*({}*) e._parameters[\"{}\"]"
                                       ,var_type
                                       ,variable_node.id_node.name.lexeme));
                if self.visiting_call_chain_literal_variable {
                    code.push_str(")");
                }
            },
            IdentifierDeclScope::EventHandlerVar => {
                code.push_str(&format!("{}",variable_node.id_node.name.lexeme));
            }
            IdentifierDeclScope::None => {
                // TODO: Explore labeling Variables as "extern" scope
                code.push_str(&format!("{}",variable_node.id_node.name.lexeme));
            },            // Actions?
            _ => panic!("Illegal scope."),
        }

        code
    }

    //* --------------------------------------------------------------------- *//

    fn format_parameter_list(&mut self,params:&Vec<ParameterNode>) {
        let mut separator = "";
        for param in params {
            self.add_code(&format!("{}", separator));
            let param_type: String = match &param.param_type_opt {
                Some(ret_type) => ret_type.clone(),
                None => String::from("<?>"),
            };
            self.add_code(&format!("{} {}", param_type, param.param_name));
            separator = ",";
        }
    }

    //* --------------------------------------------------------------------- *//

    fn format_action_name(&mut self,action_name:&String) -> String {
        return format!("{}_do",action_name);
    }

    //* --------------------------------------------------------------------- *//

    pub fn run(&mut self,system_node:&SystemNode) {
        system_node.accept( self);
    }

    //* --------------------------------------------------------------------- *//

    fn add_code(&mut self, s:&str)  {
        self.code.push_str(&*format!("{}",s));
    }

    //* --------------------------------------------------------------------- *//

    fn newline(&mut self)  {
        self.code.push_str(&*format!("\n{}",self.dent()));
    }

    //* --------------------------------------------------------------------- *//

    fn newline_to_string(&mut self, output:&mut String)  {
        output.push_str(&*format!("\n{}",self.dent()));
    }

    //* --------------------------------------------------------------------- *//

    fn dent(&self) -> String {
        return (0..self.dent).map(|_| "    ").collect::<String>()
    }

    //* --------------------------------------------------------------------- *//

    fn indent(&mut self) {
        self.dent += 1;
    }

    //* --------------------------------------------------------------------- *//

    fn outdent(&mut self) {
        self.dent -= 1;
    }

    //* --------------------------------------------------------------------- *//

    fn visit_decl_stmts(&mut self, decl_stmt_types:&Vec<DeclOrStmtType>) {
        for decl_stmt_t in decl_stmt_types.iter() {
            match decl_stmt_t {
                DeclOrStmtType::VarDeclT {var_decl_t_rc_ref}
                => {
                    let variable_decl_node = var_decl_t_rc_ref.borrow();
                    variable_decl_node.accept(self);
                },
                DeclOrStmtType::StmtT {stmt_t} => {
                    match stmt_t {
                        StatementType::ExpressionStmt { expr_stmt_t } => {
                            match expr_stmt_t {
                                ExprStmtType::ActionCallStmtT { action_call_stmt_node }
                                    => action_call_stmt_node.accept(self),                        // // TODO
                                ExprStmtType::CallStmtT { call_stmt_node }
                                    => call_stmt_node.accept(self),
                                ExprStmtType::CallChainLiteralStmtT { call_chain_literal_stmt_node }
                                    => call_chain_literal_stmt_node.accept(self),
                                ExprStmtType::AssignmentStmtT { assignment_stmt_node }
                                    => assignment_stmt_node.accept(self),
                                ExprStmtType::VariableStmtT { variable_stmt_node }
                                    => variable_stmt_node.accept(self),
                            }
                        },
                        StatementType::TransitionStmt { transition_statement } => {
                            transition_statement.accept(self);
                        }
                        StatementType::TestStmt { test_stmt_node } => {
                            test_stmt_node.accept(self);
                        },
                        StatementType::StateStackStmt {state_stack_operation_statement_node} => {
                            state_stack_operation_statement_node.accept(self);
                        },
                        StatementType::ChangeStateStmt {change_state_stmt} => {
                            change_state_stmt.accept(self);
                        },
                        StatementType::NoStmt => {
                            // TODO
                            panic!("todo");
                        }
                    }
                }
            }

        }
    }

    //* --------------------------------------------------------------------- *//

    fn generate_machinery(&mut self, system_node: &SystemNode) {
        self.newline();
        self.newline();
        self.add_code("//=============== Machinery and Mechanisms ==============//");
        self.newline();
        self.newline();
        self.add_code(&format!("public:"));
        self.newline();
        self.add_code(&format!("virtual ~{}() {{}};",system_node.name));
        self.newline();
        if let Some(first_state) = system_node.get_first_state() {
            self.newline();
            self.newline();
            self.add_code(&format!("private:"));
            self.newline();
            self.newline();
            self.add_code(&format!("typedef void ({}::*FrameState)(FrameEvent& e);",self.system_name));
            self.newline();
            self.add_code(&format!("typedef map<string,void*> FrameMap;"));
            self.newline();
            self.newline();
            self.newline();
//            self.add_code(&format!("FrameState _state_ = &{}::_s{}_;",system_node.name, first_state.borrow().name ));
            self.add_code(&format!("FrameState _state_;"));
            self.newline();
            self.add_code(&format!("StateContext* _pStateContext_;"));
            self.newline();
            self.newline();
            if self.generate_transition_state {
                if self.generate_state_context {
                    if self.generate_exit_args {
                        self.add_code(&format!("private void _transition_(FrameState newState,Dictionary<string,object> exitArgs, StateContext stateContext) {{"));
                    } else {
                        self.add_code(&format!("private void _transition_(FrameState newState, StateContext stateContext) {{"));
                    }
                } else {
                    if self.generate_exit_args {
                        self.add_code(&format!("private void _transition_(FrameState newState,Dictionary<string,object> exitArgs) {{"));
                    } else {
                        self.add_code(&format!("private void _transition_(FrameState newState) {{"));
                    }
                }
                self.indent();
                self.newline();
                if self.generate_exit_args {
                    self.add_code(&format!("FrameEvent exitEvent(\"<\",exitArgs);"));
                } else {
                    self.add_code(&format!("FrameEvent exitEvent(\"<\",nullptr);"));
                }
                self.newline();
                self.add_code(&format!("_state_ = newState;"));
                self.newline();
                self.add_code(&format!("if (_pStateContext_ && !_pStateContext_->isOnStateStack()) delete _pStateContext_;"));
                self.newline();
                if self.generate_state_context {
                    self.add_code(&format!("_pStateContext_ = pContext;"));
                    self.newline();
                    self.add_code(&format!("FrameEvent enterEvent(\">\", pContext->enterArgs);"));
                } else {
                    self.add_code(&format!("FrameEvent enterEvent(\">\",nullptr);"));
                    self.newline();
                }
                self.newline();
                self.add_code(&format!("(this->*_state_)(enterEvent);"));
                self.outdent();
                self.newline();
                self.add_code(&format!("}}"));
            }
            if self.generate_state_stack {
                self.newline();
                self.newline();
                self.add_code(&format!("std::vector<StateContext*> _stateStack_;"));
                self.newline();
                self.newline();
                self.add_code(&format!("void _stateStack_push(StateContext* pStateContext) {{"));
                self.indent();
                self.newline();
                self.add_code(&format!("pStateContext->setOnStateStack(true);"));
                self.newline();
                self.add_code(&format!("_stateStack_.push_back(pStateContext);"));
                self.outdent();
                self.newline();
                self.add_code(&format!("}}"));
                self.newline();
                self.newline();
                self.add_code(&format!("StateContext* _stateStack_pop() {{"));
                self.indent();
                self.newline();
                self.add_code(&format!("pStateContext =  _stateStack_.back();"));
                self.newline();
                self.add_code(&format!("_stateStack_.pop_back();"));
                self.newline();
                self.add_code(&format!("pStateContext->setOnStateStack(false);"));
                self.newline();
                self.add_code(&format!("return pStateContext;"));
                self.outdent();
                self.newline();
                self.add_code(&format!("}}"));
                self.newline();
            }
            if self.generate_change_state {
                self.newline();
                self.newline();
                self.add_code(&format!("private void _changeState_(FrameState newState) {{"));
                self.indent();
                self.newline();
                self.add_code(&format!("_state_ = newState;"));
                self.outdent();
                self.newline();
                self.add_code(&format!("}}"));
            }
        }
    }

    //* --------------------------------------------------------------------- *//

    fn generate_comment(&mut self,line:usize) {

        // can't use self.newline() or self.add_code() due to double borrow.
        let mut generated_comment = false;
        while self.current_comment_idx < self.comments.len() &&
            line >= self.comments[self.current_comment_idx].line {
            let comment = &self.comments[self.current_comment_idx];
            if comment.token_type == TokenType::SingleLineCommentTok {
                self.code.push_str(&*format!("  // {}",&comment.lexeme[3..]));
                self.code.push_str(&*format!("\n{}",(0..self.dent).map(|_| "\t").collect::<String>()));

            } else {
                let len = &comment.lexeme.len() - 3;
                self.code.push_str(&*format!("/* {}",&comment.lexeme[3..len]));
                self.code.push_str(&*format!("*/"));

            }

            self.current_comment_idx += 1;
            generated_comment = true;
        }
        if generated_comment {
//            self.code.push_str(&*format!("\n{}",(0..self.dent).map(|_| "\t").collect::<String>()));
        }
    }

    //* --------------------------------------------------------------------- *//

    // TODO
    fn generate_state_ref_change_state(&mut self, change_state_stmt_node: &ChangeStateStatementNode) {

        let target_state_name = match &change_state_stmt_node.state_context_t {
            StateContextType::StateRef {state_context_node}
            => &state_context_node.state_ref_node.name,
            _ => {
                self.errors.push("Change state target not found.".to_string());
                "error"
            },
        };

        self.newline();
        self.add_code(&format!("_changeState_({});", self.format_target_state_name(target_state_name)));

    }

    //* --------------------------------------------------------------------- *//

    fn generate_state_ref_code(&self, target_state_name:&str) -> String {
        format!("{}",self.format_target_state_name(target_state_name))
    }

    //* --------------------------------------------------------------------- *//

    fn generate_state_ref_transition(&mut self, transition_statement: &TransitionStatementNode) {

        self.newline();
        // this provides a block to segregate StateContext declarations which
        // can overlap w/ other generated ones in child scopes which causes
        // the compiler to complain.
        // self.add_code("{");
        // self.indent();

        let target_state_name = match &transition_statement.target_state_context_t {
            StateContextType::StateRef {state_context_node} => {
                &state_context_node.state_ref_node.name
            },
            _ => {
                self.errors.push("Unknown error.".to_string());
                ""
            },
        };

        let state_ref_code = format!("&{}::_s{}_",self.system_name,&target_state_name);

        self.newline();
        match &transition_statement.label_opt {
            Some(label) => {
                self.add_code(&format!("// {}", label));
                self.newline();
            },
            None => {},
        }

        if self.generate_state_context {
            self.add_code(&format!("pStateContext = new StateContext({});", state_ref_code));
            self.newline();
        }

        // -- Exit Arguments --

        let mut has_exit_args = false;
        if let Some(exit_args) = &transition_statement.exit_args_opt {
            if exit_args.exprs_t.len() > 0 {
                has_exit_args = true;

                // Note - searching for event keyed with "State:<"
                // e.g. "S1:<"

                let mut msg: String = String::new();
                if let Some(state_name) = &self.current_state_name_opt {
                    msg = state_name.clone();
                }
                msg.push_str(":");
                msg.push_str(&self.symbol_config.exit_msg_symbol);

                if let Some(event_sym) = self.arcanium.get_event(&msg, &self.current_state_name_opt) {
                    match &event_sym.borrow().params_opt {
                        Some(event_params) => {
                            if exit_args.exprs_t.len() != event_params.len() {
                                panic!("Fatal error: misaligned parameters to arguments.")
                            }
                            let mut param_symbols_it = event_params.iter();
                            self.add_code("map<string, Attr*> exitArg;");
                            self.newline();
                            // Loop through the ARGUMENTS...
                            for expr_t in &exit_args.exprs_t {
                                // ...and validate w/ the PARAMETERS
                                match param_symbols_it.next() {
                                    Some(p) => {
                                        let param_type = match &p.param_type {
                                            Some(param_type) => param_type.clone(),
                                            None => String::from("<?>"),
                                        };
                                        let mut expr = String::new();
                                        expr_t.accept_to_string(self, &mut expr);
                                        self.add_code(&format!("exitArg.Add(string(\"{}\"),string(\"{}\"),true,new {}({}));", p.name, param_type, param_type, expr));
                                        self.newline();
                                    },
                                    None => panic!("Invalid number of arguments for \"{}\" event handler.", msg),
                                }
                            }
                        },
                        None => panic!("Fatal error: misaligned parameters to arguments."),
                    }
                } else {
                    panic!("TODO");
                }
            }
        }

        // -- Enter Arguments --

        let enter_args_opt = match &transition_statement.target_state_context_t {
            StateContextType::StateRef { state_context_node}
                => &state_context_node.enter_args_opt,
            StateContextType::StateStackPop {}
                => &None,
        };

        if let Some(enter_args) = enter_args_opt {

            // Note - searching for event keyed with "State:>"
            // e.g. "S1:>"

            let mut msg:String = String::from(target_state_name);
            msg.push_str(":");
            msg.push_str(&self.symbol_config.enter_msg_symbol);

            if let Some(event_sym) = self.arcanium.get_event(&msg,&self.current_state_name_opt) {
                match &event_sym.borrow().params_opt {
                    Some(event_params) => {
                        if enter_args.exprs_t.len() != event_params.len() {
                            panic!("Fatal error: misaligned parameters to arguments.")
                        }
                        let mut param_symbols_it =  event_params.iter();
                        for expr_t in &enter_args.exprs_t {
                            match param_symbols_it.next() {
                                Some(p) => {
                                    let param_type = match &p.param_type {
                                        Some(param_type) => param_type.clone(),
                                        None => String::from("<?>"),
                                    };
                                    let mut expr = String::new();
                                    expr_t.accept_to_string(self,&mut expr);
                                    self.add_code(&format!("pStateContext->addEnterArg(string(\"{}\"),string(\"{}\"),true,new {}({}));",p.name,param_type,param_type,expr));
                                    self.newline();
                                },
                                None => panic!("Invalid number of arguments for \"{}\" event handler.",msg),
                            }
                        }
                    },
                    None => panic!("Invalid number of arguments for \"{}\" event handler.",msg),
                }
            } else {
                self.warnings.push(format!("State {} does not have an enter event handler but is being passed parameters in a transition", &target_state_name));
            }
        }

        // -- State Arguments --

        let target_state_args_opt = match &transition_statement.target_state_context_t {
            StateContextType::StateRef { state_context_node}
                => &state_context_node.state_ref_args_opt,
            StateContextType::StateStackPop {}
                => &Option::None,
        };
//
        if let Some(state_args) = target_state_args_opt {
//            let mut params_copy = Vec::new();
            if let Some(state_sym) = self.arcanium.get_state(&target_state_name) {
                match &state_sym.borrow().params_opt {
                    Some(event_params) => {
                        let mut param_symbols_it = event_params.iter();
                        // Loop through the ARGUMENTS...
                        for expr_t in &state_args.exprs_t {
                            // ...and validate w/ the PARAMETERS
                            match param_symbols_it.next() {

                                Some(param_symbol_rcref) => {
                                    let param_symbol = param_symbol_rcref.borrow();
                                    let param_type = match &param_symbol.param_type {
                                        Some(param_type) => param_type.clone(),
                                        None => String::from("<?>"),
                                    };
                                    let mut expr = String::new();
                                    expr_t.accept_to_string(self, &mut expr);
                                    self.add_code(&format!("pStateContext->addStateArg(string(\"{}\"),string(\"{}\"),true,new {}({}));", param_symbol.name, param_type, param_type, expr));
                                    self.newline();
                                },
                                None => panic!("Invalid number of arguments for \"{}\" state parameters.", &target_state_name),
                            }
//
                        }
                    },
                    None => {}
                }
            } else {
                panic!("TODO");
            }
        } // -- State Arguments --

        // -- State Variables --

        let target_state_rcref_opt = self.arcanium.get_state(&target_state_name);

        match target_state_rcref_opt {
            Some(q) => {
//                target_state_vars = "stateVars".to_string();
                if let Some(state_symbol_rcref) = self.arcanium.get_state(&q.borrow().name) {
                    let state_symbol = state_symbol_rcref.borrow();
                    let state_node = &state_symbol.state_node.as_ref().unwrap().borrow();
                    // generate local state variables
                    if state_node.vars.is_some() {
//                        let mut separator = "";
                        for var_rcref in state_node.vars.as_ref().unwrap() {
                            let var = var_rcref.borrow();
                            let var_type = match &var.type_opt {
                                Some(var_type) => var_type.clone(),
                                None => String::from("<?>"),
                            };
                            let expr_t = var.initializer_expr_t_opt.as_ref().unwrap();
                            let mut expr_code = String::new();
                            expr_t.accept_to_string(self,&mut expr_code);
                            self.newline();
                            self.add_code(&format!("pStateContext->addStateVar(string(\"{}\"),string(\"{}\"),true,new {}({}));", var.name, var_type, var_type, expr_code));
                            self.newline();
                        }
                    }
                }
            },
            None => {
//                code = target_state_vars.clone();
            },
        }
        let exit_args = if has_exit_args {
            "exitArgs"
        } else {
            "null"
        };
        if self.generate_state_context {
            if self.generate_exit_args {
                self.add_code(&format!("_transition_({},exitArgs,pStateContext);",self.format_target_state_name(&target_state_name)));
            } else {
                self.add_code(&format!("_transition_({},pStateContext);",self.format_target_state_name(&target_state_name)));
            }
        } else {
            if self.generate_exit_args {
                self.add_code(&format!("_transition_({},exitArgs);",self.format_target_state_name(&target_state_name)));
            } else {
                self.add_code(&format!("_transition_({});",self.format_target_state_name(&target_state_name)));
            }
        }

        // see comment at top of method for purpose of this
        // self.outdent();
        // self.newline();
        // self.add_code("}");

    }

    //* --------------------------------------------------------------------- *//

    fn format_target_state_name(&self,state_name:&str) -> String {
        format!("&{}::_s{}_",self.system_name,state_name)
    }

    //* --------------------------------------------------------------------- *//

    // NOTE!!: it is *currently* disallowed to send state or event arguments to a state stack pop target

    fn generate_state_stack_pop_transition(&mut self, transition_statement: &TransitionStatementNode) {
        let mut exit_args_val = String::from("null");

        self.newline();
        match &transition_statement.label_opt {
            Some(label) => {
                self.add_code(&format!("// {}", label));
                self.newline();
            },
            None => {},
        }

        // -- Exit Arguments --

        if let Some(exit_args) = &transition_statement.exit_args_opt {
            if exit_args.exprs_t.len() > 0 {

                // Note - searching for event keyed with "State:<"
                // e.g. "S1:<"

                let mut msg: String = String::new();
                if let Some(state_name) = &self.current_state_name_opt {
                    msg = state_name.clone();
                }
                msg.push_str(":");
                msg.push_str(&self.symbol_config.exit_msg_symbol);

                if let Some(event_sym) = self.arcanium.get_event(&msg, &self.current_state_name_opt) {
                    match &event_sym.borrow().params_opt {
                        Some(event_params) => {
                            if exit_args.exprs_t.len() != event_params.len() {
                                panic!("Fatal error: misaligned parameters to arguments.")
                            }
                            let mut param_symbols_it = event_params.iter();
                            self.add_code("FrameEventParams exitArgs = new FrameEventParams();");
                            self.newline();
                            // Loop through the ARGUMENTS...
                            for expr_t in &exit_args.exprs_t {
                                // ...and validate w/ the PARAMETERS
                                match param_symbols_it.next() {
                                    Some(p) => {
                                        let param_type = match &p.param_type {
                                            Some(param_type) => param_type.clone(),
                                            None => String::from("<?>"),
                                        };
                                        let mut expr = String::new();
                                        expr_t.accept_to_string(self, &mut expr);
                                        self.add_code(&format!("exitArgs[\"{}\"] = {};", p.name, expr));
                                        self.newline();
                                    },
                                    None => panic!("Invalid number of arguments for \"{}\" event handler.", msg),
                                }
                            }
                        },
                        None => panic!("Fatal error: misaligned parameters to arguments."),
                    }
                } else {
                    panic!("TODO");
                }
            }
        }

        if self.generate_state_context {
            self.add_code(&format!("StateContext* pStateContext = _stateStack_pop_();"));
        } else {
            self.add_code(&format!("FrameState* pState = _stateStack_pop_();"));
        }
 //       self.add_code(&format!("StateContext* pStateContext = _stateStack_pop();"));
        self.newline();
        if self.generate_exit_args {
            if self.generate_state_context {
                self.add_code(&format!("_transition_(pStateContext->state,exitArgs,pStateContext);"));
            } else {
                self.add_code(&format!("_transition_(pState,exitArgs);"));
            }
        } else {
            if self.generate_state_context {
                self.add_code(&format!("_transition_(pStateContext->state,pStateContext);"));
            } else {
                self.add_code(&format!("_transition_(pState);"));
            }
        }
    }
}

//* --------------------------------------------------------------------- *//

impl AstVisitor for CppVisitor {

    //* --------------------------------------------------------------------- *//

    fn visit_system_node(&mut self, system_node: &SystemNode) -> AstVisitorReturnType {
        self.system_name = system_node.name.clone();
        self.add_code(&format!("// {}",self.compiler_version));
        self.newline();
        self.add_code("// get include files at https://github.com/frame-lang/frame-ancillary-files");
        self.newline();
        self.newline();
        self.add_code(&format!("class {} {{", system_node.name));
        self.newline();
        self.indent();
        self.newline();
        self.add_code(&format!("class StateContext;"));
        self.newline();
        self.newline();
        self.add_code(&format!("public:"));
        self.newline();
        self.newline();

        let mut has_states = false;

        // First state name needed for machinery.
        // Don't generate if there isn't at least one state.
        match (&system_node).get_first_state() {
            Some(x) => {
                self.first_state_name = x.borrow().name.clone();
                has_states = true;
            },
            None => {},
        }

        // TODO: initialize start state context.
        if self.has_states {
            self.add_code(&format!("{}() {{", system_node.name));
            self.indent();
            self.newline();
            self.add_code(&format!("_state_ = &{}::_s{}_;", system_node.name, self.first_state_name));
            if self.generate_state_context {
                self.newline();
                self.add_code(&format!("_pStateContext_ = new StateContext(_state_);"));
                if has_states {
                    if let Some(state_symbol_rcref) = self.arcanium.get_state(&self.first_state_name) {
                        self.newline();
                        let state_symbol = state_symbol_rcref.borrow();
                        let state_node = &state_symbol.state_node.as_ref().unwrap().borrow();
                        // generate local state variables
                        if state_node.vars.is_some() {
                            for var_rcref in state_node.vars.as_ref().unwrap() {
                                let var = var_rcref.borrow();
                                let var_type = match &var.type_opt {
                                    Some(var_type) => var_type.clone(),
                                    None => String::from("<?>"),
                                };
                                let expr_t = var.initializer_expr_t_opt.as_ref().unwrap();
                                let mut expr_code = String::new();
                                expr_t.accept_to_string(self, &mut expr_code);
                                self.add_code(&format!("_stateContext_->addStateVar(\"{}\",\"{}\");", var.name, expr_code));
                                self.newline();
                            }
                        }
                    }

                }
            }

            self.outdent();
            self.newline();
            self.add_code(&format!("}}"));
            self.newline();
        }

        if let Some(interface_block_node) = &system_node.interface_block_node_opt {
            interface_block_node.accept(self);
        }

        if let Some(machine_block_node) = &system_node.machine_block_node_opt {
            machine_block_node.accept(self);
        }

        if let Some(actions_block_node) = &system_node.actions_block_node_opt {
            actions_block_node.accept(self);
        }

        if let Some(domain_block_node) = &system_node.domain_block_node_opt {
            domain_block_node.accept(self);
        }

        if self.has_states {
            self.generate_machinery(system_node);
        }

        // TODO: add comments back
        // self.newline();
        // self.generate_comment(system_node.line);
        // self.newline();
        self.outdent();
 //       self.generate_comment(system_node.line);
        self.newline();
        self.add_code("}");
        self.newline();


        AstVisitorReturnType::SystemNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_interface_block_node(&mut self, interface_block_node: &InterfaceBlockNode) -> AstVisitorReturnType {
        self.newline();
        self.add_code("//===================== Interface Block ===================//");
        self.newline();

        for interface_method_node in &interface_block_node.interface_methods {
            interface_method_node.accept(self);
        }

        AstVisitorReturnType::InterfaceBlockNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_interface_method_node(&mut self, interface_method_node: &InterfaceMethodNode) -> AstVisitorReturnType {

        self.newline();
        let return_type = match &interface_method_node.return_type {
            Some(ret) => ret.clone(),
            None => "void".to_string(),
        };

        // see if an alias exists.
        let method_name_or_alias: &String;

        match &interface_method_node.alias {
            Some(alias_message_node) => {
                method_name_or_alias = &alias_message_node.name;
            },
            None => {
                method_name_or_alias = &interface_method_node.name;
            }
        }

        self.add_code(&format!("{} {}(",return_type, interface_method_node.name));

        match &interface_method_node.params {
            Some (params)
            =>  self.format_parameter_list(params).clone(),
            None => {},
        }

        self.add_code(") {");
        self.indent();
        let mut params_param_code;
        if interface_method_node.params.is_some() {
            params_param_code = String::from("&params");
            self.newline();
            self.add_code("map<string,void *> params;");
            match &interface_method_node.params {
                Some(params) => {
                    let mut separator = "";
                    for param in params {
                        let pname = &param.param_name;
                        self.newline();
                        self.add_code(&format!("params[\"{}\"] = (void*) &{};\n", pname, pname));
                        separator = ",";
                    }
                },
                None => {}
            }
        } else {
            params_param_code = String::from("nullptr");
        }

        self.newline();
        self.add_code(&format!("FrameEvent e(string(\"{}\"),{});", method_name_or_alias,params_param_code));
        self.newline();
        self.add_code(&format!("(this->*_state_)(e);"));

        match &interface_method_node.return_type {
            Some(return_type) => {
                self.newline();
                self.add_code(&format!("return ({}) e.ret;",return_type));
            },
            None => {}
        }

        self.outdent(); self.newline();
        self.add_code(&format!("}}"));
        self.newline();

        AstVisitorReturnType::InterfaceMethodNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_machine_block_node(&mut self, machine_block_node: &MachineBlockNode) -> AstVisitorReturnType {
        self.newline();
        self.newline();
        self.add_code("//===================== Machine Block ===================//");
        self.newline();
        self.newline();
        self.add_code("private:");

        for state_node_rcref in &machine_block_node.states {
            state_node_rcref.borrow().accept(self);
        }

        AstVisitorReturnType::MachineBlockNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_actions_block_node(&mut self, actions_block_node: &ActionsBlockNode) -> AstVisitorReturnType {
        self.newline();
        self.newline();
        self.add_code("//===================== Actions Block ===================//");
        self.newline();
        self.newline();
        self.add_code("protected:");
        self.newline();

        for action_decl_node_rcref in &actions_block_node.actions {
            let action_decl_node = action_decl_node_rcref.borrow();
            action_decl_node.accept(self);
        }

        AstVisitorReturnType::ActionBlockNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_domain_block_node(&mut self, domain_block_node: &DomainBlockNode) -> AstVisitorReturnType {
        self.newline();
        self.newline();
        self.add_code("//===================== Domain Block ===================//");
        self.newline();
        self.newline();

        let mut newline = false;
        for variable_decl_node_rcref in &domain_block_node.member_variables {
            let variable_decl_node = variable_decl_node_rcref.borrow();

            if newline {
                self.newline();
            }
            variable_decl_node.accept(self);
            newline = true;
        }

        AstVisitorReturnType::DomainBlockNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_state_node(&mut self, state_node: &StateNode) -> AstVisitorReturnType {

        self.generate_comment(state_node.line);
        self.current_state_name_opt = Some(state_node.name.clone());
        self.newline();
        self.newline();
        self.add_code(&format!("void _s{}_(FrameEvent& e) {{", state_node.name));
        self.indent();

        let state_symbol = match self.arcanium.get_state(&state_node.name) {
            Some(state_symbol) => state_symbol,
            None => panic!("TODO"),
        };

        if let Some(calls) = &state_node.calls {
            for call in calls {
                self.newline();
                call.accept(self);
                self.add_code(";");
            }
        }

        self.first_event_handler = true; // context for formatting

        if state_node.evt_handlers_rcref.len() > 0 {
            for evt_handler_node in &state_node.evt_handlers_rcref {
                evt_handler_node.as_ref().borrow().accept(self);
            }
        }

        match &state_node.dispatch_opt {
            Some(dispatch) => {
                dispatch.accept(self);
            },
            None => {},
        }

        self.outdent();
        self.newline();
        self.add_code("}");

        self.current_state_name_opt = None;
        AstVisitorReturnType::StateNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_event_handler_node<'b>(& mut self, evt_handler_node: &EventHandlerNode) -> AstVisitorReturnType {
        self.current_event_ret_type = evt_handler_node.get_event_ret_type();
        self.newline();
        self.generate_comment(evt_handler_node.line);
//        let mut generate_final_close_paren = true;
        if let MessageType::CustomMessage {message_node} = &evt_handler_node.msg_t {
            if self.first_event_handler {
                self.add_code(&format!("if (e._message == \"{}\") {{", message_node.name));
            } else {
                self.add_code(&format!("else if (e._message == \"{}\") {{", message_node.name));
            }
        } else { // AnyMessage ( ||* )
            if self.first_event_handler {
                // This logic is for when there is only the catch all event handler ||*
                self.add_code(&format!("if (true) {{"));
            } else {
                // other event handlers preceded ||*
                self.add_code(&format!("else {{"));
            }
        }
        self.generate_comment(evt_handler_node.line);

        self.indent();
        if evt_handler_node.event_handler_has_transition && self.generate_state_context {
            self.newline();
            self.add_code(&format!("auto pStateContext = null;"));
        }

        match &evt_handler_node.msg_t {
            MessageType::CustomMessage {..} => {
                // Note: this is a bit convoluted as we cant use self.add_code() inside the
                // if statements as it is a double borrow (sigh).

                let params_code: Vec<String> = Vec::new();

                // NOW add the code. Sheesh.
                for param_code in params_code {
                    self.newline();
                    self.add_code(&param_code);
                }
            },
            _ => {}

        }
        //
        // if let MessageType::CustomMessage {message_node} = &evt_handler_node.msg_t {
        //
        //     let (_, msg) = EventSymbol::get_event_msg(&self.symbol_config, &Some(evt_handler_node.state_name.clone()), &message_node.name);
        //
        //     // Note: this is a bit convoluted as we cant use self.add_code() inside the
        //     // if statements as it is a double borrow (sigh).
        //
        //     let params_code: Vec<String> = Vec::new();
        //
        //     // NOW add the code. Sheesh.
        //     for param_code in params_code {
        //         self.newline();
        //         self.add_code(&param_code);
        //     }
        // }


        // Generate statements
        self.visit_decl_stmts(&evt_handler_node.statements);

        let terminator_node = &evt_handler_node.terminator_node;
        terminator_node.accept(self);
        self.outdent();

        self.newline();
        self.add_code(&format!("}}"));

        // this controls formatting here
        self.first_event_handler = false;
        self.current_event_ret_type = String::new();

        AstVisitorReturnType::EventHandlerNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_event_handler_terminator_node(&mut self, evt_handler_terminator_node: &TerminatorExpr) -> AstVisitorReturnType {
        self.newline();

        match &evt_handler_terminator_node.terminator_type {
            TerminatorType::Return => {
                match &evt_handler_terminator_node.return_expr_t_opt {
                    Some(expr_t) => {
                        self.add_code(&format!("e._return = (void*) new {}(",self.current_event_ret_type));
                        expr_t.accept(self);
                        self.add_code(");");
                        self.newline();
                        self.add_code("return;");
                        self.newline();
                    },
                    None => self.add_code("return;"),
                }

            },
            TerminatorType::Continue => {
                // self.add_code("break;")
            },
        }

        AstVisitorReturnType::EventHandlerTerminatorNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_call_statement_node(&mut self, method_call_statement: &CallStmtNode) -> AstVisitorReturnType {
        self.newline();
        method_call_statement.call_expr_node.accept(self);
        self.add_code(&format!(";"));

        AstVisitorReturnType::CallStatementNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_call_expression_node(&mut self, method_call: &CallExprNode) -> AstVisitorReturnType {

        if let Some(call_chain) = &method_call.call_chain {

            for callable in call_chain {
                callable.callable_accept(self);
                self.add_code(&format!("."));
            }
        }

        self.add_code(&format!("{}", method_call.identifier.name.lexeme));

        method_call.call_expr_list.accept(self);

        self.add_code(&format!(""));


        AstVisitorReturnType::CallExpressionNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_call_expression_node_to_string(&mut self, method_call: &CallExprNode, output:&mut String) -> AstVisitorReturnType {

        if let Some(call_chain) = &method_call.call_chain {

            for callable in call_chain {
                callable.callable_accept(self);
                output.push_str(&format!("."));
            }
        }

        output.push_str(&format!("{}", method_call.identifier.name.lexeme));

        method_call.call_expr_list.accept_to_string(self, output);

        output.push_str(&format!(""));

        AstVisitorReturnType::CallExpressionNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_call_expr_list_node(&mut self, call_expr_list: &CallExprListNode) -> AstVisitorReturnType {

        let mut separator = "";
        self.add_code(&format!("("));

        for expr in &call_expr_list.exprs_t {

            self.add_code(&format!("{}",separator));
            expr.accept(self);
            separator = ",";
        }

        self.add_code(&format!(")"));

        AstVisitorReturnType::CallExprListNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_call_expr_list_node_to_string(&mut self, call_expr_list: &CallExprListNode, output:&mut String) -> AstVisitorReturnType {

        let mut separator = "";
        output.push_str(&format!("("));

        for expr in &call_expr_list.exprs_t {

            output.push_str(&format!("{}",separator));
            expr.accept_to_string(self, output);
            separator = ",";
        }

        output.push_str(&format!(")"));

        AstVisitorReturnType::CallExprListNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_action_call_expression_node(&mut self, action_call: &ActionCallExprNode) -> AstVisitorReturnType {

        let action_name = self.format_action_name(&action_call.identifier.name.lexeme);
        self.add_code(&format!("{}", action_name));

        action_call.call_expr_list.accept(self);

        AstVisitorReturnType::ActionCallExpressionNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_action_call_expression_node_to_string(&mut self, action_call: &ActionCallExprNode, output:&mut String) -> AstVisitorReturnType {

        let action_name = self.format_action_name(&action_call.identifier.name.lexeme);
        output.push_str(&format!("{}",action_name));

        action_call.call_expr_list.accept_to_string(self, output);

        AstVisitorReturnType::ActionCallExpressionNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_action_call_statement_node(&mut self, action_call_stmt_node: &ActionCallStmtNode) -> AstVisitorReturnType {
        self.newline();
        action_call_stmt_node.action_call_expr_node.accept(self);
        self.add_code(&format!(";"));

        AstVisitorReturnType::ActionCallStatementNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_transition_statement_node(&mut self, transition_statement: &TransitionStatementNode) -> AstVisitorReturnType {

        match &transition_statement.target_state_context_t {
            StateContextType::StateRef { state_context_node}
                => self.generate_state_ref_transition(transition_statement),
            StateContextType::StateStackPop {}
                => self.generate_state_stack_pop_transition(transition_statement),
        };

        AstVisitorReturnType::CallStatementNode {}
    }


    //* --------------------------------------------------------------------- *//

    fn visit_state_ref_node(&mut self, state_ref: &StateRefNode) -> AstVisitorReturnType {
        self.add_code(&format!("{}", state_ref.name));

        AstVisitorReturnType::StateRefNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_change_state_statement_node(&mut self, change_state_stmt_node:&ChangeStateStatementNode) -> AstVisitorReturnType {

        match &change_state_stmt_node.state_context_t {
            StateContextType::StateRef { state_context_node}
                => self.generate_state_ref_change_state(change_state_stmt_node),
            StateContextType::StateStackPop {}
                => panic!("TODO - not implemented"),
        };

        AstVisitorReturnType::ChangeStateStmtNode {}
    }

    //* --------------------------------------------------------------------- *//

    // TODO: ??
    fn visit_parameter_node(&mut self, parameter_node: &ParameterNode) -> AstVisitorReturnType {

        // self.add_code(&format!("{}",parameter_node.name));

        AstVisitorReturnType::ParameterNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_dispatch_node(&mut self, dispatch_node: &DispatchNode) -> AstVisitorReturnType {
        self.newline();
        self.add_code(&format!("_s{}_(e);", dispatch_node.target_state_ref.name));
        self.generate_comment(dispatch_node.line);
        self.newline();

        AstVisitorReturnType::DispatchNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_test_statement_node(&mut self, test_stmt_node: &TestStatementNode) -> AstVisitorReturnType {

        match &test_stmt_node.test_t {
            TestType::BoolTest {bool_test_node}  => {
                bool_test_node.accept(self);
            },
            TestType::StringMatchTest {string_match_test_node} => {
                string_match_test_node.accept(self);
            },
            TestType::NumberMatchTest {number_match_test_node} => {
                number_match_test_node.accept(self);
            },
        }

        AstVisitorReturnType::TestStatementNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_bool_test_node(&mut self, bool_test_node:&BoolTestNode) -> AstVisitorReturnType {

        let mut if_or_else_if = "if ";

        self.newline();
        for branch_node in &bool_test_node.conditional_branch_nodes {
            if branch_node.is_negated {
                self.add_code(&format!("{}(!(",if_or_else_if));
            } else {
                self.add_code(&format!("{}(",if_or_else_if));
            }

            branch_node.expr_t.accept(self);

            if branch_node.is_negated {
                self.add_code(&format!(")"));
            }
            self.add_code(&format!(") {{"));
            self.indent();

            branch_node.accept(self);

            self.outdent(); self.newline();
            self.add_code(&format!("}}"));

            if_or_else_if = " else if ";
        }

        // (':' bool_test_else_branch)?
        if let Some(bool_test_else_branch_node) = &bool_test_node.else_branch_node_opt {
            bool_test_else_branch_node.accept(self);
        }

        AstVisitorReturnType::BoolTestNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_call_chain_literal_statement_node(&mut self, method_call_chain_literal_stmt_node:&CallChainLiteralStmtNode) -> AstVisitorReturnType {

        self.newline();
        method_call_chain_literal_stmt_node.call_chain_literal_expr_node.accept(self);
        self.add_code(&format!(";"));
        AstVisitorReturnType::CallChainLiteralStmtNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_call_chain_literal_expr_node(&mut self, method_call_chain_expression_node: &CallChainLiteralExprNode) -> AstVisitorReturnType {
        // TODO: maybe put this in an AST node

        let mut separator = "";

        for node in &method_call_chain_expression_node.call_chain {
            self.add_code(&format!("{}",separator));
            match &node {
                CallChainLiteralNodeType::IdentifierNodeT { id_node }=> {
                    id_node.accept(self);
                },
                CallChainLiteralNodeType::CallT {call}=> {
                    call.accept(self);
                },
                CallChainLiteralNodeType::ActionCallT {action_call_expr_node}=> {
                    action_call_expr_node.accept(self);
                },
                CallChainLiteralNodeType::VariableNodeT {var_node}=> {
                    self.visiting_call_chain_literal_variable = true;
                    var_node.accept(self);
                    self.visiting_call_chain_literal_variable = false;
                },
            }
            separator = ".";
        }

        AstVisitorReturnType::CallChainLiteralExprNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_call_chain_literal_expr_node_to_string(&mut self, method_call_chain_expression_node:&CallChainLiteralExprNode, output:&mut String) -> AstVisitorReturnType {
        let mut separator = "";

        for node in &method_call_chain_expression_node.call_chain {
            output.push_str(&format!("{}",separator));
            match &node {
                CallChainLiteralNodeType::IdentifierNodeT { id_node }=> {
                    id_node.accept_to_string(self,output);
                },
                CallChainLiteralNodeType::CallT {call}=> {
                    call.accept_to_string(self, output);
                },
                CallChainLiteralNodeType::ActionCallT {action_call_expr_node}=> {
                    action_call_expr_node.accept_to_string(self, output);
                },
                CallChainLiteralNodeType::VariableNodeT {var_node}=> {
                    var_node.accept_to_string(self, output);
                },
            }
            separator = ".";
        }
        AstVisitorReturnType::CallChainLiteralExprNode {}
    }


    //* --------------------------------------------------------------------- *//

    fn visit_bool_test_conditional_branch_node(&mut self, bool_test_true_branch_node:&BoolTestConditionalBranchNode) -> AstVisitorReturnType {

        self.visit_decl_stmts(&bool_test_true_branch_node.statements);

        match &bool_test_true_branch_node.branch_terminator_expr_opt {
            Some(branch_terminator_expr) => {
                self.newline();
                match &branch_terminator_expr.terminator_type {
                    Return => {
                        match &branch_terminator_expr.return_expr_t_opt {
                            Some(expr_t) => {
                                self.add_code(&format!("e._return = (void*) new {}(",self.current_event_ret_type));
                                expr_t.accept(self);
                                self.add_code(");");
                                self.newline();
                                self.add_code("return;");
                            },
                            None => self.add_code("return;"),
                        }
                    }
                    Continue => {
                        self.add_code("break;");
                    }
                }
            }
            None => {}
        }

        AstVisitorReturnType::BoolTestConditionalBranchNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_bool_test_else_branch_node(&mut self, bool_test_else_branch_node:&BoolTestElseBranchNode) -> AstVisitorReturnType {

        self.add_code(&format!(" else {{"));
        self.indent();

        self.visit_decl_stmts(&bool_test_else_branch_node.statements);


        match &bool_test_else_branch_node.branch_terminator_expr_opt {
            Some(branch_terminator_expr) => {
                self.newline();
                match bool_test_else_branch_node {
                    Return => {
                        match &branch_terminator_expr.return_expr_t_opt {
                            Some(expr_t) => {
                                self.add_code(&format!("e._return = (void*) new {}(",self.current_event_ret_type));
                                expr_t.accept(self);
                                self.add_code(");");
                                self.newline();
                                self.add_code("return;");
                            },
                            None => self.add_code("return;"),
                        }
                    }
                    Continue => {
                        self.add_code("break;");
                    }
                }
            }
            None => {}
        }

        self.outdent();
        self.newline();
        self.add_code(&format!("}}"));

        AstVisitorReturnType::BoolTestElseBranchNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_string_match_test_node(&mut self, string_match_test_node:&StringMatchTestNode) -> AstVisitorReturnType {

        let mut if_or_else_if = "if";

        self.newline();
        for match_branch_node in &string_match_test_node.match_branch_nodes {
            self.add_code(&format!("{} (", if_or_else_if));
            // TODO: use string_match_test_node.expr_t.accept(self) ?
            match &string_match_test_node.expr_t {
                ExprType::CallExprT { call_expr_node: method_call_expr_node }
                    => method_call_expr_node.accept(self),
                ExprType::ActionCallExprT { action_call_expr_node }
                    => action_call_expr_node.accept(self),
                ExprType::CallChainLiteralExprT { call_chain_expr_node }
                    => call_chain_expr_node.accept(self),
                ExprType::VariableExprT { var_node: id_node }
                    => id_node.accept(self),

                _ => panic!("TODO"),
            }

            // TODO: use accept
            // self.add_code(&format!(" == \""));
            // match_branch_node.string_match_pattern_node.accept(self);
            // self.add_code(&format!("\") {{"));

            let mut first_match = true;
            for match_string in &match_branch_node.string_match_pattern_node.match_pattern_strings {
                if first_match {
                    self.add_code(&format!(" == \"{}\")",match_string));
                    first_match = false;
                } else {
                    self.add_code(&format!(" || ("));
                    match &string_match_test_node.expr_t {
                        ExprType::CallExprT { call_expr_node: method_call_expr_node }
                            => method_call_expr_node.accept(self),
                        ExprType::ActionCallExprT { action_call_expr_node }
                            => action_call_expr_node.accept(self),
                        ExprType::CallChainLiteralExprT { call_chain_expr_node }
                            => call_chain_expr_node.accept(self),
                        ExprType::VariableExprT { var_node: id_node }
                            => id_node.accept(self),
                        _ => panic!("TODO"),
                    }
                    self.add_code(&format!(" == \"{}\")",match_string));
                }
            }
            self.add_code(&format!(" {{"));
            self.indent();

            match_branch_node.accept(self);

            self.outdent(); self.newline();
            self.add_code(&format!("}}"));

            if_or_else_if = " else if";
        }

        // (':' string_test_else_branch)?
        if let Some(string_match_else_branch_node) = &string_match_test_node.else_branch_node_opt {
            string_match_else_branch_node.accept(self);
        }

        AstVisitorReturnType::StringMatchTestNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_string_match_test_match_branch_node(&mut self, string_match_test_match_branch_node:&StringMatchTestMatchBranchNode) -> AstVisitorReturnType {

        self.visit_decl_stmts(&string_match_test_match_branch_node.statements);

        match &string_match_test_match_branch_node.branch_terminator_expr_opt {
            Some(branch_terminator_expr) => {
                self.newline();
                match &string_match_test_match_branch_node {
                    Return => {
                        match &branch_terminator_expr.return_expr_t_opt {
                            Some(expr_t) => {
                                self.add_code(&format!("e._return = (void*) new {}(",self.current_event_ret_type));
                                expr_t.accept(self);
                                self.add_code(");");
                                self.newline();
                                self.add_code("return;");
                            },
                            None => self.add_code("return;"),
                        }
                    }
                    Continue => {
                        self.add_code("break;");

                    }
                }
            }
            None => {}
        }

        AstVisitorReturnType::StringMatchTestMatchBranchNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_string_match_test_else_branch_node(&mut self, string_match_test_else_branch_node:&StringMatchTestElseBranchNode) -> AstVisitorReturnType {

        self.add_code(&format!(" else {{"));
        self.indent();

        self.visit_decl_stmts(&string_match_test_else_branch_node.statements);

        // TODO - factor this out to work w/ other terminator code.
        match &string_match_test_else_branch_node.branch_terminator_expr_opt {
            Some(branch_terminator_expr) => {
                self.newline();
                match string_match_test_else_branch_node {
                    Return => {
                        match &branch_terminator_expr.return_expr_t_opt {
                            Some(expr_t) => {
                                self.add_code(&format!("e._return = (void*) new {}(",self.current_event_ret_type));
                                expr_t.accept(self);
                                self.add_code(");");
                                self.newline();
                                self.add_code("return;");
                            },
                            None => self.add_code("return;"),
                        }
                    }
                    Continue => {
                        self.add_code("break;");

                    }
                }
            }
            None => {}
        }

        self.outdent();
        self.newline();
        self.add_code(&format!("}}"));

        AstVisitorReturnType::StringMatchElseBranchNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_string_match_test_pattern_node(&mut self, string_match_test_else_branch_node:&StringMatchTestPatternNode) -> AstVisitorReturnType {

        // TODO
        panic!("todo");
//        AstVisitorReturnType::StringMatchTestPatternNode {}
    }

    //-----------------------------------------------------//

    fn visit_number_match_test_node(&mut self, number_match_test_node:&NumberMatchTestNode) -> AstVisitorReturnType {

        let mut if_or_else_if = "if";

        self.newline();
        for match_branch_node in &number_match_test_node.match_branch_nodes {
            self.add_code(&format!("{} (", if_or_else_if));
            match &number_match_test_node.expr_t {
                ExprType::CallExprT { call_expr_node: method_call_expr_node }
                    => method_call_expr_node.accept(self),
                ExprType::ActionCallExprT { action_call_expr_node }
                    => action_call_expr_node.accept(self),
                ExprType::CallChainLiteralExprT { call_chain_expr_node }
                    => call_chain_expr_node.accept(self),
                ExprType::VariableExprT { var_node: id_node }
                    => id_node.accept(self),
                _ => panic!("TODO"),
            }

            let mut first_match = true;
            for match_number in &match_branch_node.number_match_pattern_nodes {
                if first_match {
                    self.add_code(&format!(" == {})",match_number.match_pattern_number));
                    first_match = false;
                } else {
                    self.add_code(&format!(" || ("));
                    match &number_match_test_node.expr_t {
                        ExprType::CallExprT { call_expr_node: method_call_expr_node }
                            => method_call_expr_node.accept(self),
                        ExprType::ActionCallExprT { action_call_expr_node }
                            => action_call_expr_node.accept(self),
                        ExprType::CallChainLiteralExprT { call_chain_expr_node }
                            => call_chain_expr_node.accept(self),
                        ExprType::VariableExprT { var_node: id_node }
                            => id_node.accept(self),
                        _ => panic!("TODO"),
                    }
                    self.add_code(&format!(" == {})",match_number.match_pattern_number));
                }
            }

            self.add_code(&format!(") {{"));
            self.indent();

            match_branch_node.accept(self);

            self.outdent(); self.newline();
            self.add_code(&format!("}}"));

            //           self.indent();

            if_or_else_if = " else if";
        }

        // (':' number_test_else_branch)?
        if let Some(number_match_else_branch_node) = &number_match_test_node.else_branch_node_opt {
            number_match_else_branch_node.accept(self);
        }


        AstVisitorReturnType::NumberMatchTestNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_number_match_test_match_branch_node(&mut self, number_match_test_match_branch_node:&NumberMatchTestMatchBranchNode) -> AstVisitorReturnType {

        self.visit_decl_stmts(&number_match_test_match_branch_node.statements);

        // TODO - factor this out to work w/ other terminator code.
        match &number_match_test_match_branch_node.branch_terminator_expr_opt {
            Some(branch_terminator_expr) => {
                self.newline();
                match number_match_test_match_branch_node {
                    Return => {
                        match &branch_terminator_expr.return_expr_t_opt {
                            Some(expr_t) => {
                                self.add_code(&format!("e._return = (void*) new {}(",self.current_event_ret_type));
                                expr_t.accept(self);
                                self.add_code(");");
                                self.newline();
                                self.add_code("return;");
                            },
                            None => self.add_code("return;"),
                        }
                    }
                    Continue => {
                        self.add_code("break;");

                    }
                }
            }
            None => {}
        }

        AstVisitorReturnType::NumberMatchTestMatchBranchNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_number_match_test_else_branch_node(&mut self, number_match_test_else_branch_node:&NumberMatchTestElseBranchNode) -> AstVisitorReturnType {

        self.add_code(&format!(" else {{"));
        self.indent();

        self.visit_decl_stmts(&number_match_test_else_branch_node.statements);


        match &number_match_test_else_branch_node.branch_terminator_expr_opt {
            Some(branch_terminator_expr) => {
                self.newline();
                match number_match_test_else_branch_node {
                    Return => {
                        match &branch_terminator_expr.return_expr_t_opt {
                            Some(expr_t) => {
                                self.add_code(&format!("e._return = (void*) new {}(",self.current_event_ret_type));
                                expr_t.accept(self);
                                self.add_code(");");
                                self.newline();
                                self.add_code("return;");
                            },
                            None => self.add_code("return;"),
                        }
                    }
                    Continue => {
                        self.add_code("break;");

                    }
                }
            }
            None => {}
        }

        self.outdent();
        self.newline();
        self.add_code(&format!("}}"));

        AstVisitorReturnType::NumberMatchElseBranchNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_number_match_test_pattern_node(&mut self, match_pattern_node:&NumberMatchTestPatternNode) -> AstVisitorReturnType {
        self.add_code(&format!("{}", match_pattern_node.match_pattern_number));

        AstVisitorReturnType::NumberMatchTestPatternNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_expression_list_node(&mut self, expr_list: &ExprListNode) -> AstVisitorReturnType {

        let mut separator = "";
        self.add_code(&format!("("));
        for expr in &expr_list.exprs_t {

            self.add_code(&format!("{}",separator));
            expr.accept(self);
            separator = ",";
        }
        self.add_code(&format!(")"));

        AstVisitorReturnType::ParentheticalExpressionNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_expression_list_node_to_string(&mut self, expr_list: &ExprListNode, output:&mut String) -> AstVisitorReturnType {

//        self.add_code(&format!("{}(e);\n",dispatch_node.target_state_ref.name));

        let mut separator = "";
        output.push_str(&format!("{{"));
        for expr in &expr_list.exprs_t {

            output.push_str(&format!("{}",separator));
            expr.accept_to_string(self, output);
            separator = ",";
        }
        output.push_str(&format!("}}"));

        AstVisitorReturnType::ParentheticalExpressionNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_literal_expression_node(&mut self, literal_expression_node: &LiteralExprNode) -> AstVisitorReturnType {

        match &literal_expression_node.token_t {
            TokenType::NumberTok
                => self.add_code(&format!("{}", literal_expression_node.value)),
            TokenType::SuperStringTok
                => self.add_code(&format!("{}", literal_expression_node.value)),
            TokenType::StringTok
                => self.add_code(&format!("\"{}\"", literal_expression_node.value)),
            TokenType::TrueTok
                => self.add_code("true"),
            TokenType::FalseTok
                => self.add_code("false"),
            TokenType::NullTok
                => self.add_code("null"),
            TokenType::NilTok
                => self.add_code("null"),
            _ => panic!("TODO"),
        }

        AstVisitorReturnType::ParentheticalExpressionNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_literal_expression_node_to_string(&mut self, literal_expression_node: &LiteralExprNode, output:&mut String) -> AstVisitorReturnType {

        // TODO: make a focused enum or the literals
        match &literal_expression_node.token_t {
            TokenType::NumberTok => {
                output.push_str(&format!("{}", literal_expression_node.value))
            },
            TokenType::StringTok => {
                output.push_str(&format!("\"{}\"", literal_expression_node.value));
            },
            TokenType::TrueTok => {
                output.push_str("true");
            },
            TokenType::FalseTok => {
                output.push_str("false");
            },
            TokenType::NilTok => {
                output.push_str("null");
            },
            TokenType::NullTok => {
                output.push_str("null");
            },
            _ => panic!("TODO"),
        }

        AstVisitorReturnType::ParentheticalExpressionNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_identifier_node(&mut self, identifier_node: &IdentifierNode) -> AstVisitorReturnType {

        self.add_code(&format!("{}",identifier_node.name.lexeme));

        AstVisitorReturnType::IdentifierNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_identifier_node_to_string(&mut self, identifier_node: &IdentifierNode, output:&mut String) -> AstVisitorReturnType {

        output.push_str(&format!("{}",identifier_node.name.lexeme));

        AstVisitorReturnType::IdentifierNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_state_stack_operation_node(&mut self, state_stack_operation_node:&StateStackOperationNode) -> AstVisitorReturnType {

//        self.add_code(&format!("{}",identifier_node.name.lexeme));

        AstVisitorReturnType::StateStackOperationNode {}
    }


    //* --------------------------------------------------------------------- *//

    fn visit_state_stack_operation_node_to_string(&mut self, state_stack_operation_node:&StateStackOperationNode, output:&mut String) -> AstVisitorReturnType {

//        self.add_code(&format!("{}",identifier_node.name.lexeme));

        AstVisitorReturnType::StateStackOperationNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_state_stack_operation_statement_node(&mut self, state_stack_op_statement_node:&StateStackOperationStatementNode) -> AstVisitorReturnType {

//        self.add_code(&format!("{}",identifier_node.name.lexeme));

        //       panic!("TODO: how is this used?");

        match state_stack_op_statement_node.state_stack_operation_node.operation_t {
            StateStackOperationType::Push => {
                self.newline();
                if self.generate_state_context {
                    self.add_code(&format!("_stateStack_push_(_state_context_);"));
                } else {
                    self.add_code(&format!("_stateStack_push_(_state_);"));
                }
            },
            StateStackOperationType::Pop => {
                if self.generate_state_context {
                    self.add_code(&format!("_pStateStack_ = _stateStack_pop_()"));
                } else {
                    self.add_code(&format!("let state = _stateStack_pop_()"));
                }
            }
        }
        AstVisitorReturnType::StateStackOperationStatementNode {}
    }
    //* --------------------------------------------------------------------- *//

    fn visit_state_context_node(&mut self, state_context_node:&StateContextNode) -> AstVisitorReturnType {

        // TODO
//        self.add_code(&format!("{}",identifier_node.name.lexeme));

        AstVisitorReturnType::StateContextNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_frame_event_part(&mut self, frame_event_part:&FrameEventPart) -> AstVisitorReturnType {

//        self.add_code(&format!("{}",identifier_node.name.lexeme));

        // TODO: make this code generate from settings
        match frame_event_part {
            FrameEventPart::Event => self.add_code(&format!("e")),
            FrameEventPart::Message => self.add_code(&format!("e._message")),
            FrameEventPart::Param {param_tok} => self.add_code(&format!("e._params[\"{}\"]",param_tok.lexeme)),
            FrameEventPart::Return => self.add_code(&format!("e._return")),
        }

        AstVisitorReturnType::FrameEventExprType {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_frame_event_part_to_string(&mut self, frame_event_part:&FrameEventPart, output:&mut String) -> AstVisitorReturnType {

        // TODO: make this code generate from settings
        match frame_event_part {
            FrameEventPart::Event => output.push_str("e"),
            FrameEventPart::Message => output.push_str("e._message"),
            FrameEventPart::Param {param_tok} => output.push_str(&format!("e._params[\"{}\"]",param_tok.lexeme)),
            FrameEventPart::Return => output.push_str("e._return"),
        }

        AstVisitorReturnType::FrameEventExprType {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_action_decl_node(&mut self, action_decl_node: &ActionDeclNode) -> AstVisitorReturnType {

        self.newline();
        let action_ret_type:String = match &action_decl_node.type_opt {
            Some(ret_type) => ret_type.clone(),
            None => String::from("void"),
        };

        let action_name = self.format_action_name(&action_decl_node.name);
        self.add_code(&format!("virtual {} {}(",action_ret_type, action_name));

        match &action_decl_node.params {
            Some (params)
                =>  self.format_parameter_list(params).clone(),
            None => {},
        }

        self.add_code(&format!(") {{}}"));

        AstVisitorReturnType::ActionDeclNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_variable_decl_node(&mut self, variable_decl_node: &VariableDeclNode) -> AstVisitorReturnType {

        let var_type = match &variable_decl_node.type_opt {
            Some(x) => x.clone(),
            None => String::from("<?>"),
        };
        let var_name =  &variable_decl_node.name;
        let var_init_expr = &variable_decl_node.initializer_expr_t_opt.as_ref().unwrap();
        self.newline();
        let mut code = String::new();
        var_init_expr.accept_to_string(self, &mut code);
        self.add_code( &format!("{} {} = {};",var_type,var_name, code));

        AstVisitorReturnType::VariableDeclNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_variable_expr_node(&mut self, variable_node: &VariableNode) -> AstVisitorReturnType {
        let code = self.format_variable_expr(variable_node);
        self.add_code(&code);

        AstVisitorReturnType::AssignmentExprNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_variable_expr_node_to_string(&mut self, variable_node: &VariableNode, output:&mut String) -> AstVisitorReturnType {
        let code = self.format_variable_expr(variable_node);
        output.push_str(&code);

        AstVisitorReturnType::AssignmentExprNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_variable_stmt_node(&mut self, variable_stmt_node: &VariableStmtNode) -> AstVisitorReturnType {
        // TODO: what is this line about?
        self.generate_comment(variable_stmt_node.get_line());
        self.newline();
        let code = self.format_variable_expr(&variable_stmt_node.var_node);
        self.add_code(&code);

        AstVisitorReturnType::AssignmentExprNode {}
    }


    //* --------------------------------------------------------------------- *//

    fn visit_assignment_expr_node(&mut self, assignment_expr_node: &AssignmentExprNode) -> AstVisitorReturnType {

        self.generate_comment(assignment_expr_node.line);
        self.newline();
        assignment_expr_node.l_value_box.accept(self);
        self.add_code(" = ");
        assignment_expr_node.r_value_box.accept(self);
        self.add_code(";");

        AstVisitorReturnType::AssignmentExprNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_assignment_expr_node_to_string(&mut self, assignment_expr_node: &AssignmentExprNode, output:&mut String) -> AstVisitorReturnType {

        self.generate_comment(assignment_expr_node.line);
        self.newline();
        self.newline_to_string(output);
        assignment_expr_node.l_value_box.accept_to_string(self, output);
        output.push_str(" = ");
        assignment_expr_node.r_value_box.accept_to_string(self, output);
        output.push_str(";");

        AstVisitorReturnType::AssignmentExprNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_assignment_statement_node(&mut self, assignment_stmt_node: &AssignmentStmtNode) -> AstVisitorReturnType {

        self.generate_comment(assignment_stmt_node.get_line());
        assignment_stmt_node.assignment_expr_node.accept(self);

        AstVisitorReturnType::AssignmentExprNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_unary_expr_node(&mut self, unary_expr_node: &UnaryExprNode) -> AstVisitorReturnType {

        // TODO
        //       self.generate_comment(assignment_expr_node.line);
        unary_expr_node.operator.accept(self);
        unary_expr_node.right_rcref.borrow().accept(self);

        AstVisitorReturnType::UnaryExprNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_unary_expr_node_to_string(&mut self, unary_expr_node: &UnaryExprNode, output:&mut String) -> AstVisitorReturnType {

        // TODO
        //       self.generate_comment(assignment_expr_node.line);
        unary_expr_node.operator.accept_to_string(self, output);
        unary_expr_node.right_rcref.borrow().accept_to_string(self,output);

        AstVisitorReturnType::UnaryExprNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_binary_expr_node(&mut self, binary_expr_node: &BinaryExprNode) -> AstVisitorReturnType {

        // TODO
        //       self.generate_comment(assignment_expr_node.line);
        if binary_expr_node.operator == OperatorType::LogicalXor {
            self.add_code("((");
            binary_expr_node.left_rcref.borrow().accept(self);
            self.add_code(") && !(");
            binary_expr_node.right_rcref.borrow().accept(self);
            self.add_code(")) || (!(");
            binary_expr_node.left_rcref.borrow().accept(self);
            self.add_code(") && (");
            binary_expr_node.right_rcref.borrow().accept(self);
            self.add_code("))");
        } else {
            binary_expr_node.left_rcref.borrow().accept(self);
            binary_expr_node.operator.accept(self);
            binary_expr_node.right_rcref.borrow().accept(self);
        }

        AstVisitorReturnType::BinaryExprNode {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_binary_expr_node_to_string(&mut self, binary_expr_node: &BinaryExprNode, output:&mut String) -> AstVisitorReturnType {

        if binary_expr_node.operator == OperatorType::LogicalXor {
            output.push_str("((");
            binary_expr_node.left_rcref.borrow().accept_to_string(self,output);
            output.push_str(") && !(");
            binary_expr_node.right_rcref.borrow().accept_to_string(self,output);
            output.push_str(")) || (!(");
            binary_expr_node.left_rcref.borrow().accept_to_string(self,output);
            output.push_str(") && (");
            binary_expr_node.right_rcref.borrow().accept_to_string(self,output);
            output.push_str("))");

        } else {
            binary_expr_node.left_rcref.borrow().accept_to_string(self,output);
            binary_expr_node.operator.accept_to_string(self, output);
            binary_expr_node.right_rcref.borrow().accept_to_string(self,output);
        }
        AstVisitorReturnType::BinaryExprNode {}
    }


    //* --------------------------------------------------------------------- *//

    fn visit_operator_type(&mut self, operator_type: &OperatorType) -> AstVisitorReturnType {

        match operator_type {
            OperatorType::Plus => self.add_code(" + "),
            OperatorType::Minus => self.add_code(" - "),
            OperatorType::Negated => self.add_code("-"),
            OperatorType::Multiply => self.add_code(" * "),
            OperatorType::Divide => self.add_code(" / "),
            OperatorType::Greater => self.add_code(" > "),
            OperatorType::GreaterEqual => self.add_code(" >= "),
            OperatorType::Less => self.add_code(" < "),
            OperatorType::LessEqual => self.add_code(" <= "),
            OperatorType::Not => self.add_code("!"),
            OperatorType::EqualEqual => self.add_code(" == "),
            OperatorType::NotEqual => self.add_code(" != "),
            OperatorType::LogicalAnd => self.add_code(" && "),
            OperatorType::LogicalOr => self.add_code(" || "),
            OperatorType::LogicalXor => self.add_code(""),
        }

        AstVisitorReturnType::OperatorType {}
    }

    //* --------------------------------------------------------------------- *//

    fn visit_operator_type_to_string(&mut self, operator_type: &OperatorType, output:&mut String) -> AstVisitorReturnType {

        match operator_type {
            OperatorType::Plus => output.push_str(" + "),
            OperatorType::Minus => output.push_str(" - "),
            OperatorType::Negated => output.push_str("-"),
            OperatorType::Multiply => output.push_str(" * "),
            OperatorType::Divide => output.push_str(" / "),
            OperatorType::Greater => output.push_str(" > "),
            OperatorType::GreaterEqual => output.push_str(" >= "),
            OperatorType::Less => output.push_str(" < "),
            OperatorType::LessEqual => output.push_str(" <= "),
            OperatorType::Not => output.push_str("!"),
            OperatorType::EqualEqual => output.push_str(" == "),
            OperatorType::NotEqual => output.push_str(" != "),
            OperatorType::LogicalAnd => output.push_str(" && "),
            OperatorType::LogicalOr => output.push_str(" || "),
            OperatorType::LogicalXor => output.push_str(""),
        }

        AstVisitorReturnType::OperatorType {}
    }
}



