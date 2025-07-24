use capnp::dynamic_value;
use fill_random_values::Filler;

capnp::generated_code!(pub mod addressbook_capnp);
capnp::generated_code!(pub mod fill_capnp);

pub fn main() {
    let mut message = ::capnp::message::Builder::new_default();
    let mut addressbook = message.init_root::<addressbook_capnp::address_book::Builder>();

    let mut filler = Filler::new(::rand::rng(), 10);
    let dynamic: dynamic_value::Builder = addressbook.reborrow().into();
    filler.fill(dynamic.downcast()).unwrap();

    println!("{:#?}", addressbook.into_reader());
}
