use quote::quote;
use syn::{parse_quote, Expr, ExprClosure, File, Item};

static WRAP_FUNC_QED: &str = "Wrap function exists .qed";

/// Sanitize and format guard expressions for use in Mermaid diagrams.
///
/// When the `prettyplease` feature is enabled, this uses prettyplease to format
/// the expression in idiomatic Rust style. Otherwise, it uses basic sanitization
/// to make the expression safe for Mermaid.
pub fn sanitize_expr(expr: &Expr) -> String {
	match expr {
		Expr::Closure(closure) => sanitize_closure(closure),
		other_expr => {
			let content = quote! { #other_expr}.to_string();
			as_mermaid_guard(&content)
		},
	}
}

#[cfg(feature = "prettyplease")]
pub fn sanitize_closure(expr: &ExprClosure) -> String {
	// Wrap the expression in a function that returns it, so prettyplease can format it
	let body = &expr.body;
	let item: Item = parse_quote! {
	  fn __guard_expr() -> bool {
		#body
	  }
	};
	let file = File { shebang: None, attrs: Vec::new(), items: vec![item] };

	let content = prettyplease::unparse(&file);
	// Extract just the expression part from the function body
	// The format will be: "fn __guard_expr() -> bool {\n    EXPR\n}\n"
	let start = content.find('{').expect(WRAP_FUNC_QED);
	let end = content.rfind('}').expect(WRAP_FUNC_QED);

	as_mermaid_guard(&content[start + 1..end])
}

#[cfg(not(feature = "prettyplease"))]
pub fn sanitize_closure(expr: &ExprClosure) -> String {
	let content = quote! { #expr }.to_string();
	as_mermaid_guard(&content)
}

fn as_mermaid_guard(expr_str: &str) -> String {
	// Remove newlines and excessive whitespace
	let expr_str = expr_str.split_whitespace().collect::<Vec<_>>().join(" ");
	format!("[{}]", expr_str.replace(":", ""))
}
