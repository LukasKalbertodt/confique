//! Finding the end of a Rust expression without parsing it.
//!
//! I will likely extract this as a stand-alone crate at some point. Otherwise,
//! it might seem a bit overkill for just confique.

use proc_macro2::{Spacing, TokenTree};


/// Keywords after which an operand follows (as opposed to an operator).
const PREFIX_KEYWORDS: &[&str] = &[
    "return", "break", "in", "if", "while", "match", "let", "move", "async", "mut", "const",
];

/// Finds the end of a Rust expression in a comma separated list, without
/// actually parsing the expression.
///
/// Pass the tokens to [`Self::push`] one by one. A comma ends the expression
/// if [`Self::at_top_level`] returns `true` when reaching it.
///
/// Commas inside of `()`, `[]` and `{}` are no problem, as each of those is a
/// single token tree. Apart from that, commas only occur in two places inside
/// of an expression: in generic arguments (`foo::<A, B>()`) and in closure
/// parameters (`|a, b| a + b`). So those two are skipped. The tricky part is
/// that `<` and `|` are also binary operators (`a < b`). They can be told
/// apart by their position: generic arguments and closures only start where
/// an operand is expected, binary operators only where an operand just ended.
/// The exception are types (as in `x as Foo<A, B>`), where `<` always starts
/// generic arguments.
#[derive(Default)]
pub(crate) struct ExprScanner {
    /// Nesting depth of `<...>`, i.e. generic arguments.
    angle_depth: u32,

    /// Inside the `|...|` of a closure.
    closure_params: bool,

    /// Inside a type, where `<` always starts generic arguments. Types follow
    /// `as` and `->` (the return type of a closure).
    in_type: bool,

    /// An operand just ended, so an operator is expected next and not another
    /// operand.
    after_operand: bool,

    /// The previous token, if it was punctuation.
    prev: Option<(char, Spacing)>,
}

impl ExprScanner {
    /// Returns `true` if a comma at the current position is not part of the
    /// expression, i.e. ends it.
    pub(crate) fn at_top_level(&self) -> bool {
        self.angle_depth == 0 && !self.closure_params
    }

    /// Advances by one token.
    pub(crate) fn push(&mut self, tt: &TokenTree) {
        let punct = match tt {
            TokenTree::Punct(p) => Some((p.as_char(), p.spacing())),
            _ => None,
        };
        let prev = std::mem::replace(&mut self.prev, punct);
        let prev_is = |c| prev.map(|(prev, _)| prev) == Some(c);

        // Closure parameters and generic arguments are skipped.
        if self.closure_params {
            self.closure_params = !matches!(punct, Some(('|', _)));
            return;
        }
        if self.angle_depth > 0 {
            match punct {
                Some(('<', _)) => self.angle_depth += 1,

                // Unless it's the `>` of `->`, e.g. in `Fn(A) -> B`.
                Some(('>', _)) if !prev_is('-') => {
                    self.angle_depth -= 1;
                    self.after_operand = true;
                }

                // The `<` after a type was in fact a `<=`: `x as u8 <= y`.
                Some(('=', _)) if prev_is('<') => {
                    self.angle_depth -= 1;
                    self.in_type = false;
                    self.after_operand = false;
                }
                _ => {}
            }
            return;
        }

        match tt {
            TokenTree::Punct(p) => {
                // `<` and `|` only start generic arguments or a closure where
                // an operand is expected, and not as second char of `<<`
                // or `||`.
                let c = p.as_char();
                let starts_operand = !self.after_operand && prev != Some((c, Spacing::Joint));
                match c {
                    '<' if self.in_type || starts_operand => self.angle_depth = 1,
                    '|' if starts_operand => self.closure_params = true,
                    '>' if prev_is('-') => self.in_type = true,

                    // These can be part of a type: paths, lifetimes,
                    // references and pointers.
                    ':' | '\'' => {}
                    '&' | '*' | '!' if !self.after_operand => {}
                    _ => self.in_type = false,
                }
                self.after_operand = c == '?';
            }

            // Lifetime or label
            TokenTree::Ident(_) if prev_is('\'') => self.after_operand = false,
            TokenTree::Ident(ident) => {
                let ident = ident.to_string();
                self.in_type |= ident == "as";
                self.after_operand = ident != "as" && !PREFIX_KEYWORDS.contains(&&*ident);
            }

            // Attribute `#[...]`
            TokenTree::Group(_) if prev_is('#') => {}
            TokenTree::Group(_) | TokenTree::Literal(_) => {
                self.in_type = false;
                self.after_operand = true;
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use proc_macro2::{TokenStream, TokenTree};

    use super::ExprScanner;


    /// Splits a comma separated list of expressions.
    fn split(src: &str) -> Vec<String> {
        let mut out = vec![TokenStream::new()];
        let mut scanner = ExprScanner::default();
        for tt in TokenStream::from_str(src).unwrap() {
            let is_comma = matches!(&tt, TokenTree::Punct(p) if p.as_char() == ',');
            if is_comma && scanner.at_top_level() {
                out.push(TokenStream::new());
                scanner = ExprScanner::default();
            } else {
                scanner.push(&tt);
                out.last_mut().unwrap().extend([tt]);
            }
        }
        out.iter().map(|tokens| tokens.to_string()).collect()
    }

    /// Asserts that the given tokens are treated as a single expression. That
    /// they actually are one is checked by rustc itself, via `$e:expr`.
    macro_rules! check {
        ($e:expr) => { check(stringify!($e)) };
    }

    #[track_caller]
    fn check(expr: &str) {
        let normalized = TokenStream::from_str(expr).unwrap().to_string();
        let expected = [&*normalized, "0", &*normalized];
        assert_eq!(split(&format!("{expr}, 0, {expr}")), expected);
    }

    #[test]
    fn splits_at_commas() {
        assert_eq!(split("a, b"), ["a", "b"]);
        assert_eq!(split("a < b, c > d"), ["a < b", "c > d"]);
        assert_eq!(split("f(a, b), [c, d], { e, f }"), ["f (a , b)", "[c , d]", "{ e , f }"]);
        assert_eq!(split("a,"), ["a", ""]);
    }

    #[test]
    fn generic_args() {
        check!(iter.collect::<HashMap<_, _>>());
        check!(HashMap::<&str, u32>::new());
        check!(foo::<A, [u8; 2], B>());
        check!(Foo::<A, B> { x: 1 });
        check!(x.parse::<Foo<A, B>>()?.bar::<C, D>());
        check!(f::<fn(A, B) -> C, D>());
        check!(f::<dyn Fn(A) -> B, C>());
        check!(f::<{ 1 + 2 }, B>());
        check!(f::<-1, B>());
        check!(f::<'a, B>());
        check!(f::<A, B>() < g::<C, D>());
        check!(T::<A, B>::X < y);
        check!(size_of::<(A, B)>() << 2);
    }

    #[test]
    fn qualified_paths() {
        check!(<HashMap<String, u32>>::new());
        check!(<Foo as Bar<A, B>>::baz());
        check!(<<A as B<C, D>>::E as F<G, H>>::i());
        check!(1 + <T as Tr<A, B>>::X);
        check!(-<f64 as From<f32, u8>>::from(1.0));
        check!(&<Vec<A, B>>::new());
        check!(&mut <Vec<A, B>>::new());
        check!(!<Vec<A, B>>::CONST);
        check!(a..<Foo<A, B>>::MAX);
        check!(a == <Foo<A, B>>::MAX);
        check!(a <= <Foo<A, B>>::MAX);
        check!(a < <Foo<A, B>>::MAX);
        check!(a << <Foo<A, B>>::MAX);
        check!(x += <Foo<A, B>>::X);
    }

    /// `<` and `>` that are operators.
    #[test]
    fn comparisons_and_shifts() {
        check!(a < b);
        check!(a << b);
        check!(a <<= b);
        check!(a < b && c > d);
        check!(a.0 < b.1);
        check!(a? < b);
        check!(foo() < bar());
        check!(a[0] < b[1]);
        check!(x.await < y);
        check!("a" < "b");
        check!(1 < 2);
        check!(self < other);
        check!(*a < *b);
        check!(!a < b);
        check!((a as u8) < b);
        check!(if a < b { 1 } else { 2 });
        check!(if a { b } else { c } < d);
        check!(match a < b { true => 1, false => 2 });
        check!([a < b, c > d].len());
    }

    #[test]
    fn casts() {
        check!(x as Alias<A, B>);
        check!(x as *const Foo<A, B>);
        check!(x as *mut *const Foo<A, B>);
        check!(x as &dyn Tr<A, B>);
        check!(x as &'static mut Foo<A, B>);
        check!(x as <T as Tr<A, B>>::Out);
        check!(x as Foo<A, B> as Bar<C, D>);
        check!(x as a::b::Foo<A, B>::Bar<C, D> + 1 < 2);
        check!(x as fn(A, B) -> Foo<C, D>);
        check!(a as u8 + b < c);
        check!(a as u8 & b < c);
        check!(a as u8 * b < c);
        check!(x as u64 <= MAX);
        check!(x as *const _ <= end);
        check!(x as Foo<A, B> <= y);
    }

    #[test]
    fn closures() {
        check!(|a, b| a + b);
        check!(|a: Foo<A, B>, b| a + b);
        check!(|a, b| -> Foo<A, B> { a });
        check!(|(a, b), [c, d]| a);
        check!(|a, b| |c, d| a + c);
        check!(move |a, b| a);
        check!(async move |a, b| a);
        check!(|| 1);
        check!(|| -> Foo<A, B> { x });
        check!(&|a, b| a);
        check!(!|a, b| true);
        check!(x = |a, b| a);
        check!(x || |a, b| a);
        check!(x | |a, b| a);
        check!(foo(|a, b| a < b, c));
    }

    /// `|` that is an operator or part of a pattern.
    #[test]
    fn bit_or() {
        check!(x || y);
        check!(x | y);
        check!(x |= y);
        check!(if let Some(1) | Some(2) = x { 1 } else { 2 });
        check!(if let A::<X, Y>(a) | B(a) = x { 1 } else { 2 });
    }

    #[test]
    fn after_keywords_labels_and_attributes() {
        check!(return |a, b| a);
        check!(break 'l |a, b| a);
        check!(break 'l <Foo<A, B>>::new());
        check!('a: loop { break 'a <Foo<A, B>>::X });
        check!(if let <Foo<A, B>>::X = y { 1 } else { 2 });
        check!(match <Foo<A, B>>::X { _ => 1 });
        check!(while <Foo<A, B>>::X { });
        check!(for x in <Foo<A, B>>::iter() { });
        check!(#[allow(unused)] |a, b| a);
        check!(#[cfg(a, b)] <Foo<A, B>>::new());
    }
}
