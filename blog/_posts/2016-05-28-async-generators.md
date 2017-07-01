---
layout: post
title: async generators
author: dwrensha
---

Until recently,
the concept of *generators*, or *resumable functions*,
seemed to me like a cute idea
with only niche use cases.
Sure, I had heard that generators
in [python](https://www.python.org/dev/peps/pep-0255/)
and
[javascript](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Iterators_and_Generators#Generators)
could make certain things much nicer,
but how often does one really need to have
what appears to be nothing more than a fancy iterator?
It wasn't until I followed
this [Rust RFC thread](https://github.com/rust-lang/rfcs/issues/1081)
that the true potential of generators in Rust started to dawn on me.
Today they are my personal number-one most-desired new feature for the language, mostly because I
believe they are the biggest missing piece in Rust's async story.

But how are generators relevant to async at all?

To understand that, let's first imagine what generators might look like in Rust.
We start with a trait definition (borrowed from eddyb's comment in the above-linked thread):

{% highlight rust %}
pub enum Yield<T, R> {
    Value(T),
    Return(R)
}

pub trait Generator {
    type Value;
    type Return;
    fn next(&mut self) -> Yield<Self::Value, Self::Return>;
}
{% endhighlight %}

So a generator looks a lot like an iterator. You can ask it for the next value
until you reach its end, signalled by `Return`.
The interesting part is that there is a special way to construct
values that implement `Generator`. For example:

{% highlight rust %}
fn fib_with_sum(n: usize) -<u64>-> u64 {
     let mut a = 0;
     let mut b = 1;
     let mut sum = 0;
     for _ in 0..n {
         yield a; // <--- new keyword "yield"
         sum += a;
         let next = a + b;
         a = b;
         b = next;
     }
     return sum;
}
{% endhighlight %}

Here I've used some strawman syntax `fn foo() -<T>-> R`, which denotes that `foo`
is a *generator function* that produces some number of `T` values
and then finishes by producing an `R`.
The `yield` keyword inside of a generator function means that a `Yield::Value()` should be produced
and the generator should be paused.
When you call `fib_with_sum()`, the value you get back is a generator,
which can be used by calling `next()`, like this:
{% highlight rust %}
let mut g = fib_with_sum(5);
g.next(); // Yield::Value(0)
g.next(); // Yield::Value(1)
g.next(); // Yield::Value(1)
g.next(); // Yield::Value(2)
g.next(); // Yield::Value(3)
g.next(); // Yield::Return(7)
{% endhighlight %}

Another thing that we might want to do is to
have one generator delegate to another generator.
The `yield from` construction allows that:

{% highlight rust %}
fn sub_gen(n: u64) -<u64>-> bool {
    for i in 0..n {
        yield i
    }
    return n == 3;
}

fn gen() -<u64>-> () {
   for j in 0.. {
       if yield from sub_gen(j) {
           return j;
       }
   }
}
{% endhighlight %}

Running this gives:
{% highlight rust %}
let mut g = gen();
g.next(); // Yield::Value(0)
g.next(); // Yield::Value(0)
g.next(); // Yield::Value(1)
g.next(); // Yield::Value(0)
g.next(); // Yield::Value(1)
g.next(); // Yield::Value(2)
g.next(); // Yield::Return(3)
{% endhighlight %}


Note that when the sub-generator is done,
its return value gets plugged in at the `yield from` expression of the
calling generator. So `gen()` continues until `j == 3`.

### Using generators for async I/O

Now for the punchline. Using generators, we can define
asynchronous reader and writer traits like this:

{% highlight rust %}

use std::io::{Error, ErrorKind, Result};

enum AsyncStatus {
  Fd(FileDescriptor),
  Timeout(Time),
  //...
}

pub trait AsyncWrite {
    /// Attempts to write all buf.len() bytes from buf into the
    /// stream. Returns once all of the bytes have been written.
    fn write(&mut self, bytes: &[u8]) -<AsyncStatus>-> Result<()>;
}

pub trait AsyncRead {
    /// Attempts to read buf.len() bytes from the stream,
    /// writing them into buf. Returns the number of bytes
    /// actually read. Returns as soon as min_bytes are
    /// read or EOF is encountered.
    fn try_read(&mut self,
                buf: &mut [u8],
                min_bytes: usize)
        -<AsyncStatus>-> Result<usize>;

    /// Like try_read(), but returns an error if EOF is
    /// encountered before min_bytes can be read.
    fn read(&mut self,
            buf: &mut [u8],
            min_bytes: usize)
        -<AsyncStatus>-> Result<usize>;
    {
       let n = try!(yield from self.try_read(buf, min_bytes));
       if n < min_bytes {
           Err(Error::new(ErrorKind::UnexpectedEof,
                          format!("expected {} but got {} bytes",
                                  min_bytes, n)))
       } else {
           Ok(n)
       }
    }
}

{% endhighlight %}

Then, at the top level of our program, we have
a task executor, where
{% highlight rust %}
type Task = Generator<Value=AsyncStatus, Return=()>;
{% endhighlight %}
The task executor owns a collection of tasks and is responsible
for running them when they are ready.
When a task runs and needs to pend on some I/O, it yields
back what it's currently waiting on, for example a file descriptor.
When none of the tasks can make any progress, the executor calls
a OS-specific API like `kevent()` to wait until more progress can be made.

Unlike with fibers/green threading, it remains very clear where
switches-between-tasks can take place.
Unlike with promises, we don't have to be constantly allocating closures
on the heap. Generator-based async-I/O seems like an all-around win!

[(discussion on /r/rust)](https://www.reddit.com/r/rust/comments/4li9v2/generators_are_the_missing_piece_for_async/)
