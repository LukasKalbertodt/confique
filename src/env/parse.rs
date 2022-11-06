//! Functions for the `#[config(parse_env = ...)]` attribute.

use std::str::FromStr;

/// Splits the environment variable by separator `SEP`, parses each element
/// with [`FromStr`] and collects everything via [`FromIterator`].
///
/// To avoid having to specify the separator via `::<>` syntax, see the
/// other functions in this module.
///
/// [`FromStr`]: std::str::FromStr
/// [`FromIterator`]: std::iter::FromIterator
///
///
/// # Example
///
/// ```
/// use confique::Config;
///
/// #[derive(Debug, confique::Config)]
/// struct Conf {
///     #[config(env = "PORTS", parse_env = confique::env::parse::list_by_sep::<',', _, _>)]
///     ports: Vec<u16>,
/// }
///
/// std::env::set_var("PORTS", "8080,8000,8888");
/// let conf = Conf::builder().env().load()?;
/// assert_eq!(conf.ports, vec![8080, 8000, 8888]);
/// # Ok::<_, confique::Error>(())
/// ```
pub fn list_by_sep<const SEP: char, T, C>(input: &str) -> Result<C, <T as FromStr>::Err>
where
    T: FromStr,
    C: FromIterator<T>,
{
    input.split(SEP).map(T::from_str).collect()
}


macro_rules! specify_fn_wrapper {
    ($fn_name:ident, $sep:literal) => {
        #[doc = concat!("Like [`list_by_sep`] with `", $sep, "` separator.")]
        pub fn $fn_name<T, C>(input: &str) -> Result<C, <T as FromStr>::Err>
        where
            T: FromStr,
            C: FromIterator<T>,
        {
            list_by_sep::<$sep, _, _>(input)
        }
    }
}

specify_fn_wrapper!(list_by_comma, ',');
specify_fn_wrapper!(list_by_semicolon, ';');
specify_fn_wrapper!(list_by_colon, ':');
specify_fn_wrapper!(list_by_space, ' ');
