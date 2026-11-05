use capnp::private::capability::InvocationOrigin;

/// One slot per request; no registry or ambient execution state.
#[derive(Default)]
pub(crate) struct Slot {
    context: Option<InvocationOrigin>,
    conflicted: bool,
}

impl Slot {
    pub(crate) fn set(&mut self, context: InvocationOrigin) -> capnp::Result<()> {
        if self.conflicted || self.context.as_ref().is_some_and(|old| !old.eq(&context)) {
            self.conflicted = true;
            self.context = None;
            return Err(Self::conflict());
        }
        self.context = Some(context);
        Ok(())
    }

    pub(crate) fn take(self) -> capnp::Result<Option<InvocationOrigin>> {
        if self.conflicted {
            Err(Self::conflict())
        } else {
            Ok(self.context)
        }
    }

    fn conflict() -> capnp::Error {
        capnp::Error::failed("conflicting local call context".into())
    }
}
