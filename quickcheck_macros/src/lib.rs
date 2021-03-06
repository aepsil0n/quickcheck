//! This crate provides the `#[quickcheck]` attribute. Its use is
//! documented in the `quickcheck` crate.

#![crate_name = "quickcheck_macros"]
#![crate_type = "dylib"]
#![license = "MIT/ASL2"]
#![doc(html_root_url = "http://burntsushi.net/rustdoc/quickcheck")]

#![feature(plugin_registrar)]

extern crate syntax;
extern crate rustc;

use syntax::ast;
use syntax::codemap;
use syntax::parse::token;
use syntax::ext::base::{ExtCtxt, Modifier};
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;

use rustc::plugin::Registry;

/// For the `#[quickcheck]` attribute. Do not use.
#[plugin_registrar]
#[doc(hidden)]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_syntax_extension(token::intern("quickcheck"),
                                  Modifier(box expand_meta_quickcheck));
}

/// Expands the `#[quickcheck]` attribute.
///
/// Expands:
/// ```
/// #[quickcheck]
/// fn check_something(_: uint) -> bool {
///     true
/// }
/// ```
/// to:
/// ```
/// #[test]
/// fn check_something() {
///     fn check_something(_: uint) -> bool {
///         true
///     }
///     ::quickcheck::quickcheck(check_something)
/// }
/// ```
fn expand_meta_quickcheck(cx: &mut ExtCtxt,
                          span: codemap::Span,
                          _: &ast::MetaItem,
                          item: P<ast::Item>) -> P<ast::Item> {
    match item.node {
        ast::ItemFn(..) | ast::ItemStatic(..) => {
            // Copy original function without attributes
            let prop = P(ast::Item {attrs: Vec::new(), ..(*item).clone()});
            // ::quickcheck::quickcheck
            let check_ident = token::str_to_ident("quickcheck");
            let check_path = vec!(check_ident, check_ident);
            // Wrap original function in new outer function, calling ::quickcheck::quickcheck()
            let fn_decl = P(codemap::respan(span, ast::DeclItem(prop.clone())));
            let inner_fn = P(codemap::respan(span, ast::StmtDecl(fn_decl, ast::DUMMY_NODE_ID)));
            let inner_ident = cx.expr_ident(span, prop.ident);
            let check_call = cx.expr_call_global(span, check_path, vec![inner_ident]);
            let body = cx.block(span, vec![inner_fn], Some(check_call));
            let nil = P(ast::Ty {
                id: ast::DUMMY_NODE_ID,
                node: ast::TyTup(vec![]),
                span: codemap::DUMMY_SP,
            });
            let test = cx.item_fn(span, item.ident, Vec::new(), nil, body);

            // Copy attributes from original function
            let mut attrs = item.attrs.clone();
            // Add #[test] attribute
            attrs.push(cx.attribute(span, cx.meta_word(span, token::intern_and_get_ident("test"))));
            // Attach the attributes to the outer function
            P(ast::Item {attrs: attrs, ..(*test).clone()})
        },
        _ => {
            cx.span_err(span, "#[quickcheck] only supported on statics and functions");
            item
        }
    }
}
