---
layout: post
title: error handling revisited
author: dwrensha
---


Last week I pushed some changes that
switch [capnproto-rust](https://www.github.com/dwrensha/capnproto-rust)
over to using return-value-based error handling.
In particular, we no longer use the
default value fallback strategy discussed in
[this previous post]({{site.baseurl}}/2014/04/06/error-handling.html).
Now any method that might fail on malformed
input returns a `::std::result::Result<T, ::capnp::Error>`
instead of a bare `T`.

These changes remove a lot of complexity and
have allowed me to delete a significant amount of code.
They also provide, I think, a more honest interface for
users of the library.
Now the type signatures of the getter methods
make it clear exactly where input validation errors
are possible.
The `try!()` macro makes it easy enough to deal
with such errors in a principled manner.

Here is what a small example looks like after the changes:

{% highlight rust %}
pub fn print_address_book(
        address_book: address_book::Reader)
        -> ::std::result::Result<(), ::capnp::Error>
{
    for person in try!(address_book.get_people()).iter() {
        println!("{}: {}", try!(person.get_name()),
                           try!(person.get_email()));
        for phone in try!(person.get_phones()).iter() {
            let type_name = match phone.get_type() {
                Ok(person::phone_number::Type::Mobile) => "mobile",
                Ok(person::phone_number::Type::Home) => "home",
                Ok(person::phone_number::Type::Work) => "work",
                Err(::capnp::NotInSchema(n)) => "UNKNOWN",
            };
            println!("  {} phone: {}",
                     type_name, try!(phone.get_number()));
        }
        match person.get_employment().which() {
            Ok(person::employment::Unemployed(())) => {
                println!("  unemployed");
            }
            Ok(person::employment::Employer(employer)) => {
               println!("  employer: {}", try!(employer));
            }
            Ok(person::employment::School(school)) => {
                println!("  student at: {}", try!(school));
            }
            Ok(person::employment::SelfEmployed(())) => {
                println!("  self-employed");
            }
            Err(::capnp::NotInSchema(_)) => { }
        }
    }
    Ok(())
}
{% endhighlight %}


Notice that there are in fact two types of errors being dealt with here.
There is `::capnp::Error`, which gets returned
when a malformed pointer field is encountered in the encoded message.
There is also `::capnp::NotInSchema`, which indicates that
an enumerant or union discriminant value was
outside of the range defined in the schema.
The second type of error can occur if
the encoded data was constructed using a newer version of the schema.
Instead of ignoring such cases, as in the above code, we might instead
wish to propagate their errors.
Because `::capnp::Error` implements `::std::error::FromError<::capnp::NotInSchema>`,
we can easily accomplish that using the `try!()` macro:

{% highlight rust %}
            //...
            let type_name = match try!(phone.get_type()) {
                person::phone_number::Type::Mobile => "mobile",
                person::phone_number::Type::Home => "home",
                person::phone_number::Type::Work => "work",
            };
{% endhighlight %}


A year ago when I wrote the previous post on error handling,
the main reason that I decided not to go with return-value-based
error handling was that I thought it felt too heavyweight.
My sense now is that the `try!()` macro and `FromError` trait
can make things quite usable.
