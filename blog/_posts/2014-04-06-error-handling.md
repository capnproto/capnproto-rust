---
layout: post
title: error handling
author: dwrensha
---

I've recently made some significant changes
to how [capnproto-rust](https://www.github.com/dwrensha/capnproto-rust)
handles malformed values.
In the old
version, a task would fail when
it encountered a bad input.
In the new version, it instead
falls back to a default value
and logs an error.
(edit: this is no longer exactly true; see [update](#update) below.)


The changes allow capnproto-rust
to fit more nicely
within Rust's mechanisms for
memory safety,
and I think they
highlight an interesting comparison point between
Rust and C++.


Recall that
a value in [Cap'n Proto](http://kentonv.github.io/capnproto/) is just a
segmented sequence of bytes,
with exactly the same format
whether it's on the wire, on
disk, or in memory.
This uniformity of representation
means there
is no encode/decode step,
which in turn allows
Cap'n-Proto-based communication
to be
extremely fast.


Note, however, that
not all segmented sequences of bytes
are valid values,
and one consequence of
not having a decode step
is that
any
validation must occur on the fly.
As you traverse a
sequence of bytes,
you might discover something
wrong with it.
For example, you might find a struct
where you were expecting a list.

What do you do then?

The
[C++ implementation](https://www.github.com/kentonv/capnproto)
by default
throws an exception.
This lets you know immediately when something goes wrong
and avoids the need for verbose
explicit error handling.
If you want to be robust while reading messages from untrusted sources,
you can catch and recover from these exceptions.
For example, you might terminate the sender's connection
and then resume with your handling
of messages from other sources.


I originally imagined that capnproto-rust
ought to work in roughly the same way,
with Rust task failure
as a drop-in replacement for
C++ exceptions.
However, as tempting as that approach may seem, it
hits a serious hurdle: Rust's type system.
Rust guarantees
that a task can only mutate
data it owns or is currently borrowing,
and all such data is wiped out when a task fails.
Therefore, you would not be
able to put any important cumulative state
in a task that reads untrusted Cap'n Proto messages.
Instead, you would often be forced to have a separate
task that sanitizes data from Cap'n Proto messages,
largely defeating
the purpose of having no decode step.

At first glance, it may seem that Rust is being
overly restrictive here. What would be so bad
about letting us recover
some of our data after a `fail!()`?
The problem
is that it's difficult
to know statically *which*
parts of the
data are safe to recover.
After all, the reason for the failure
was probably that some internal invariant
was broken.
This is true for C++ exceptions as well, but
C++ lets you shoot yourself in the foot.
Rust chooses a simple, restrictive policy that
guarantees safety. C++ chooses a simple,
permissive policy that demands care from the programmer.

So where does that leave us?

A perhaps more idiomatic way to structure the Rust implementation would be
to have
any possibly-failing read operation explicitly return
a `Result<T,DecodeError>`.
You could then wrap all read operations in a
`DecodeResult<T>` monad, much like the `IoResult<T>` monad.
This is the first thing I tried. It works, but it feels too heavyweight.

Instead, I think the best solution for the Rust implementation
is to log an error and fall back to a default value
when invalid input is detected.
The C++ implementation
has long supported this mode of operation,
as an opt-in feature.
Traversing Cap'n Proto message remains
nearly as convenient as traversing a
native struct,
and you don't ever have to
reason about exceptional control flow
or broken internal invariants.


Finally, note that if we ever want the old behavior back,
it would be easy to add a compile-time option
that would, as before, trigger task failure on malformed
input. If all messages are from trusted sources, this may be
a sensible option.


#### update (7 April 2014) <a name="update"></a>

Based on some feedback
from [r/rust](http://www.reddit.com/r/rust/comments/22d36q/error_handling_in_capnprotorust/),
I've implemented a new plan.
Now a malformed message *does* cause task failure by default.
For cases where that behavior is unacceptable,
you can set the `fail_fast` field of `ReaderOptions` to false,
on a message-by-message basis. Doing so will
enable the default-value fall-back described above.

#### update (21 March 2015) <a name="update2"></a>

[New post]({{site.baseurl}}/2015/03/21/error-handling-revisited.html).
I've switched to explicit `Result`-based error handling.
