use super::Storage;
use actix::{dev::ToEnvelope, Addr, Handler, MailboxError, Message};
use futures::{future::Either, Future};

/// Helper for sending queries
///
/// This is alternative to `Addr<Storage>::send` which unwraps results of type `Result` using `Either` type for wrapping errors.
pub trait StorageAddrExt<A> {
    fn get_storage_addr<M, T, E>(&self) -> &Addr<A>
    where
        A: Handler<M> + Send,
        A::Context: ToEnvelope<A, M>,
        M: Message<Result = Result<T, E>> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static;

    /// Send query and get unwrapped result
    fn send_query<M, T, E>(
        &self,
        msg: M,
    ) -> Box<dyn Future<Item = T, Error = Either<MailboxError, E>> + Send>
    where
        A: Handler<M> + Send,
        A::Context: ToEnvelope<A, M>,
        M: Message<Result = Result<T, E>> + Send + 'static,
        T: Send + 'static,
        E: Send + 'static,
    {
        Box::new(
            self.get_storage_addr()
                .send(msg)
                .map_err(Either::A)
                .and_then(error_to_b),
        )
    }
}

fn error_to_b<T, E, W>(res: Result<T, E>) -> Result<T, Either<W, E>> {
    res.map_err(Either::B)
}

impl StorageAddrExt<Storage> for Addr<Storage> {
    fn get_storage_addr<M, T, E>(&self) -> &Addr<Storage>
    where
        Storage: Handler<M>,
        M: Message<Result = Result<T, E>> + Send,
        T: Send + 'static,
        E: Send + 'static,
    {
        self
    }
}
