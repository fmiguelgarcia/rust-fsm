use crate::{
    code_gen::input_param_names,
    parser::{InputVariant, OutputSpec},
};

use proc_macro2::TokenStream;
use quote::quote;
use std::mem::swap;
use syn::{Expr, Ident};

/// The full information about a state transition. Used to unify the
/// represantion of the simple and the compact forms.
#[derive(Clone)]
pub struct MatchCase<'a> {
    pub state: &'a Ident,
    pub input: &'a InputVariant,
    pub guard_expr: Option<Expr>,
    pub next_state: &'a Ident,
    pub output: Option<&'a OutputSpec>,
}

impl<'a> MatchCase<'a> {
    /// Creates a new match case for a state transition.
    pub fn new(state: &'a Ident, input: &'a InputVariant, next_state: &'a Ident) -> Self {
        Self {
            state,
            input,
            guard_expr: None,
            next_state,
            output: None,
        }
    }

    /// Sets the output specification for this transition.
    ///
    /// # Arguments
    /// * `output` - Optional output specification (constant or call expression)
    pub fn with_output(mut self, mut output: Option<&'a OutputSpec>) -> Self {
        swap(&mut self.output, &mut output);
        self
    }

    /// Adds a guard expression to conditionally enable this transition.
    ///
    /// # Arguments
    /// * `guard_expr` - The guard expression that must evaluate to true
    pub fn with_guard(mut self, guard_expr: Expr) -> Self {
        self.guard_expr = Some(guard_expr);
        self
    }

    /// Returns the constant output identifier if this transition has a constant output.
    ///
    /// Returns `None` if the output is a call expression or if there is no output.
    pub fn constant_output(&self) -> Option<&Ident> {
        self.output.and_then(|out_spec| match out_spec {
            OutputSpec::Constant(out_ident) => Some(out_ident),
            _ => None,
        })
    }

    /// Generates the input pattern matching code for this transition.
    ///
    /// Returns a token stream representing the input pattern, which may include
    /// parameter bindings if the input has fields and they're needed for guards or output.
    ///
    /// # Examples
    ///
    /// For a state machine input like:
    /// ```ignore
    /// Locked => {
    ///     Coin(u32) match amount {
    ///         0..50 => Locked [RefundInsufficient],
    ///         50.. => Unlocked
    ///     }
    /// }
    /// ```
    ///
    /// This generates patterns like:
    /// - `Self::Input::Coin(ref __arg0)` - when guard or output needs the parameter
    /// - `Self::Input::Coin(..)` - when parameters are not needed
    /// - `Self::Input::Push` - for unit variants with no fields
    fn code_gen_input_patern(&self) -> TokenStream {
        let input_name = &self.input.name;

        if !self.input.fields.is_empty() {
            // If we use any expression as guard or as output generation, we need its parameter
            // names.
            let use_param_names = self.guard_expr.is_some()
                || self.output.map(OutputSpec::is_expr).unwrap_or_default();

            if use_param_names {
                let param_names = input_param_names(self.input);
                quote! { Self::Input::#input_name(#(ref #param_names),*) }
            } else {
                quote! { Self::Input::#input_name(..) }
            }
        } else {
            quote! { Self::Input::#input_name }
        }
    }

    /// Generates the guard expression code for this transition.
    ///
    /// Returns a token stream containing the `if` condition with the guard expression,
    /// or an empty token stream if no guard is present.
    ///
    /// # Examples
    ///
    /// For guards with tuple variant inputs:
    /// ```ignore
    /// Idle => {
    ///     StartPayment(u32) if |amount: &u32| *amount >= 100 => Processing
    /// }
    /// ```
    /// Generates:
    /// ```ignore
    /// if (|amount: &u32| *amount >= 100)(__arg0)
    /// ```
    ///
    /// For guards with unit variant inputs:
    /// ```ignore
    /// Locked => {
    ///     Push if some_condition() => Unlocked
    /// }
    /// ```
    /// Generates:
    /// ```ignore
    /// if some_condition()
    /// ```
    ///
    /// For transitions without guards, returns an empty token stream.
    fn code_gen_guard_expr(&self) -> TokenStream {
        match &self.guard_expr {
            Some(guard_expr) => {
                if self.input.fields.is_empty() {
                    quote! { if #guard_expr }
                } else {
                    let param_names = input_param_names(self.input);
                    quote! { if (#guard_expr)(#(#param_names),*) }
                }
            }
            None => TokenStream::new(),
        }
    }

    /// Generates the pattern matching code for this transition case.
    ///
    /// Returns a token stream containing the complete match arm for state transitions.
    /// This combines the state pattern, input pattern, optional guard, and next state.
    ///
    /// # Examples
    ///
    /// Simple transition without guards:
    /// ```ignore
    /// Locked => {
    ///     Coin => Unlocked
    /// }
    /// ```
    /// Generates:
    /// ```ignore
    /// (Self::State::Locked, Self::Input::Coin) => Some(Self::State::Unlocked),
    /// ```
    ///
    /// Transition with guard and tuple variant:
    /// ```ignore
    /// Idle => {
    ///     StartPayment(u32) if |amount: &u32| *amount >= 100 => Processing
    /// }
    /// ```
    /// Generates:
    /// ```ignore
    /// (Self::State::Idle, Self::Input::StartPayment(ref __arg0))
    ///     if (|amount: &u32| *amount >= 100)(__arg0) => Some(Self::State::Processing),
    /// ```
    ///
    /// Transition with match guard:
    /// ```ignore
    /// Failed => {
    ///     Retry(u32) match attempts {
    ///         0..3 => Processing
    ///     }
    /// }
    /// ```
    /// Generates:
    /// ```ignore
    /// (Self::State::Failed, Self::Input::Retry(ref __arg0))
    ///     if (attempts)(__arg0) => Some(Self::State::Processing),
    /// ```
    pub fn code_gen_transition_case(&self) -> TokenStream {
        let input_pattern = self.code_gen_input_patern();
        let guard_code = self.code_gen_guard_expr();
        let state = self.state;
        let next_state = self.next_state;

        quote! { (Self::State::#state, #input_pattern) #guard_code => Some(Self::State::#next_state), }
    }

    /// Generates the output case code for this transition.
    ///
    /// Returns `Some(TokenStream)` containing the match arm for output generation,
    /// or `None` if this transition has no output specified.
    ///
    /// # Examples
    ///
    /// For constant outputs:
    /// ```ignore
    /// Locked => {
    ///     Push => Locked [AccessDenied]
    /// }
    /// ```
    /// Generates:
    /// ```ignore
    /// (Self::State::Locked, Self::Input::Push) => Some(Self::Output::AccessDenied),
    /// ```
    ///
    /// For call expression outputs:
    /// ```ignore
    /// Idle => {
    ///     Add(i32, i32) => Idle [|a: &i32, b: &i32| CalcOutput::Result(a + b)]
    /// }
    /// ```
    /// Generates:
    /// ```ignore
    /// (Self::State::Idle, Self::Input::Add(ref __arg0, ref __arg1)) =>
    ///     Some((|a: &i32, b: &i32| CalcOutput::Result(a + b))(__arg0, __arg1)),
    /// ```
    pub fn code_gen_output_case(&self) -> Option<TokenStream> {
        let output_spec = self.output?;
        let state = self.state;
        let input_pattern = self.code_gen_input_patern();
        let guard_code = self.code_gen_guard_expr();

        let ouput_gen_expr = match output_spec {
            OutputSpec::Constant(output_value) => quote! { Self::Output::#output_value },
            OutputSpec::Call(call_expr) => {
                // Generate code to call the closure with input tuple fields
                let param_names = input_param_names(self.input);
                let param_names_expr = if !param_names.is_empty() {
                    quote! { (#(#param_names),*) }
                } else {
                    TokenStream::new()
                };

                quote! { (#call_expr) #param_names_expr }
            }
        };

        let code =
            quote! { (Self::State::#state, #input_pattern) #guard_code => Some(#ouput_gen_expr), };
        Some(code)
    }
}
