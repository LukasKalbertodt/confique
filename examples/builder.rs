//! This example demonstrate how you can add a builder-API to the partial type
//! by using `layer_attr` to derive `TypedBuilder`.

use confique::Config;
use typed_builder::TypedBuilder;


#[derive(Debug, Config)]
#[config(layer_attr(derive(TypedBuilder)))]
#[config(layer_attr(builder(field_defaults(default, setter(strip_option)))))]
#[allow(dead_code)]
struct Conf {
    name: String,
    age: Option<u8>,
}

fn main() {
    let layer = <Conf as Config>::Layer::builder()
        .age(23)
        .build();
    let config = Conf::from_layer(layer);
    let _ = dbg!(config);
}
