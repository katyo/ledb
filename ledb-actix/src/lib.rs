extern crate serde;
extern crate ledb;
extern crate actix;

#[cfg(test)]
extern crate futures;

#[cfg(test)]
extern crate tokio;

#[cfg(test)]
#[macro_use]
extern crate serde_derive;

#[cfg(test)]
#[macro_use]
extern crate serde_json;

use std::marker::PhantomData;
use std::sync::Arc;
use std::path::Path;
use serde::{Serialize, de::DeserializeOwned};
use ledb::{Storage as LeStorage, Result as LeResult, Identifier};
use actix::{Actor, Addr, Message, SyncContext, SyncArbiter, Handler};

pub use ledb::{Filter, Comp, Cond, Order, OrderKind, IndexKind, KeyType, KeyData, Modify, Action, Primary, Document, DocumentsIterator};

#[derive(Clone)]
pub struct Storage {
    db: Arc<LeStorage>,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> LeResult<Self> {
        Ok(Self { db: Arc::new(LeStorage::open(path)?) })
    }

    pub fn start(self, threads: usize) -> Addr<Self> {
        SyncArbiter::start(threads, move || self.clone())
    }
}

impl Actor for Storage {
    type Context = SyncContext<Self>;
}

/// Get collections request
pub struct GetCollections;

impl Message for GetCollections {
    type Result = LeResult<Vec<String>>;
}

impl Handler<GetCollections> for Storage {
    type Result = <GetCollections as Message>::Result;

    fn handle(&mut self, _: GetCollections, _: &mut Self::Context) -> Self::Result {
        self.db.get_collections()
    }
}

/// Drop collection
#[allow(non_snake_case)]
pub fn DropCollection<C: Into<Identifier>>(coll: C) -> DropCollectionMsg {
    DropCollectionMsg(coll.into())
}

pub struct DropCollectionMsg(Identifier);

impl Message for DropCollectionMsg {
    type Result = LeResult<bool>;
}

impl Handler<DropCollectionMsg> for Storage {
    type Result = <DropCollectionMsg as Message>::Result;

    fn handle(&mut self, DropCollectionMsg(collection): DropCollectionMsg, _: &mut Self::Context) -> Self::Result {
        self.db.drop_collection(collection)
    }
}

/// Get indexes
#[allow(non_snake_case)]
pub fn GetIndexes<C: Into<Identifier>>(coll: C) -> GetIndexesMsg {
    GetIndexesMsg(coll.into())
}

pub struct GetIndexesMsg(Identifier);

impl Message for GetIndexesMsg {
    type Result = LeResult<Vec<(String, IndexKind, KeyType)>>;
}

impl Handler<GetIndexesMsg> for Storage {
    type Result = <GetIndexesMsg as Message>::Result;

    fn handle(&mut self, GetIndexesMsg(collection): GetIndexesMsg, _: &mut Self::Context) -> Self::Result {
        self.db.collection(collection)?.get_indexes()
    }
}

/// Ensure index for collection
#[allow(non_snake_case)]
pub fn EnsureIndex<C: Into<Identifier>, F: Into<Identifier>>(coll: C, field: F, kind: IndexKind, key: KeyType) -> EnsureIndexMsg {
    EnsureIndexMsg(coll.into(), field.into(), kind, key)
}

pub struct EnsureIndexMsg(Identifier, Identifier, IndexKind, KeyType);

impl Message for EnsureIndexMsg {
    type Result = LeResult<bool>;
}

impl Handler<EnsureIndexMsg> for Storage {
    type Result = <EnsureIndexMsg as Message>::Result;

    fn handle(&mut self, EnsureIndexMsg(collection, field, kind, key): EnsureIndexMsg, _: &mut Self::Context) -> Self::Result {
        self.db.collection(collection)?.ensure_index(field, kind, key)
    }
}

/// Drop index of collection
#[allow(non_snake_case)]
pub fn DropIndex<C: Into<Identifier>, F: Into<Identifier>>(coll: C, field: F) -> DropIndexMsg {
    DropIndexMsg(coll.into(), field.into())
}

pub struct DropIndexMsg(Identifier, Identifier);

impl Message for DropIndexMsg {
    type Result = LeResult<bool>;
}

impl Handler<DropIndexMsg> for Storage {
    type Result = <DropIndexMsg as Message>::Result;

    fn handle(&mut self, DropIndexMsg(collection, field): DropIndexMsg, _: &mut Self::Context) -> Self::Result {
        self.db.collection(collection)?.drop_index(field)
    }
}

/// Insert new document into collection
#[allow(non_snake_case)]
pub fn Insert<C: Into<Identifier>, T: Serialize>(coll: C, data: T) -> InsertMsg<T> {
    InsertMsg(coll.into(), data)
}

pub struct InsertMsg<T>(Identifier, T);

impl<T: Serialize> Message for InsertMsg<T> {
    type Result = LeResult<Primary>;
}

impl<T: Serialize> Handler<InsertMsg<T>> for Storage {
    type Result = <InsertMsg<T> as Message>::Result;

    fn handle(&mut self, InsertMsg(collection, document): InsertMsg<T>, _: &mut Self::Context) -> Self::Result {
        self.db.collection(collection)?.insert(&document)
    }
}

/// Store new version of the previously inserted document
#[allow(non_snake_case)]
pub fn Store<C: Into<Identifier>, T: Serialize>(coll: C, data: Document<T>) -> StoreMsg<T> {
    StoreMsg(coll.into(), data)
}

pub struct StoreMsg<T>(Identifier, Document<T>);

impl<T: Serialize> Message for StoreMsg<T> {
    type Result = LeResult<()>;
}

impl<T: Serialize> Handler<StoreMsg<T>> for Storage {
    type Result = <StoreMsg<T> as Message>::Result;

    fn handle(&mut self, StoreMsg(collection, document): StoreMsg<T>, _: &mut Self::Context) -> Self::Result {
        self.db.collection(collection)?.put(&document)
    }
}

/// Update documents using filter and modifier
#[allow(non_snake_case)]
pub fn Update<C: Into<Identifier>>(coll: C, filter: Option<Filter>, modify: Modify) -> UpdateMsg {
    UpdateMsg(coll.into(), filter, modify)
}

pub struct UpdateMsg(Identifier, Option<Filter>, Modify);

impl Message for UpdateMsg {
    type Result = LeResult<usize>;
}

impl Handler<UpdateMsg> for Storage {
    type Result = <UpdateMsg as Message>::Result;

    fn handle(&mut self, UpdateMsg(collection, filter, modify): UpdateMsg, _: &mut Self::Context) -> Self::Result {
        self.db.collection(collection)?.update(filter, modify)
    }
}

/// Remove documents using filter
#[allow(non_snake_case)]
pub fn Remove<C: Into<Identifier>>(coll: C, filter: Option<Filter>) -> RemoveMsg {
    RemoveMsg(coll.into(), filter)
}

pub struct RemoveMsg(Identifier, Option<Filter>);

impl Message for RemoveMsg {
    type Result = LeResult<usize>;
}

impl Handler<RemoveMsg> for Storage {
    type Result = <RemoveMsg as Message>::Result;

    fn handle(&mut self, RemoveMsg(collection, filter): RemoveMsg, _: &mut Self::Context) -> Self::Result {
        self.db.collection(collection)?.remove(filter)
    }
}

/// Find documents using filter and ordering
#[allow(non_snake_case)]
pub fn Find<C: Into<Identifier>, T>(coll: C, filter: Option<Filter>, order: Order) -> FindMsg<T> {
    FindMsg(coll.into(), filter, order, PhantomData)
}

pub struct FindMsg<T>(Identifier, Option<Filter>, Order, PhantomData<T>);

impl<T: DeserializeOwned + 'static> Message for FindMsg<T> {
    type Result = LeResult<DocumentsIterator<T>>;
}

impl<T: DeserializeOwned + 'static> Handler<FindMsg<T>> for Storage {
    type Result = <FindMsg<T> as Message>::Result;

    fn handle(&mut self, FindMsg(collection, filter, order, ..): FindMsg<T>, _: &mut Self::Context) -> Self::Result {
        self.db.collection(collection)?.find(filter, order)
    }
}

#[cfg(test)]
mod tests {
    use std::fs::remove_dir_all;
    use serde_json::{from_value};
    use futures::{Future};
    use tokio::{spawn};
    use actix::{System};
    use super::{Storage, EnsureIndex, Insert, Find, IndexKind, KeyType};

    macro_rules! json_val {
        ($($json:tt)+) => {
            from_value(json!($($json)+)).unwrap()
        };
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct BlogPost {
        pub title: String,
        pub tags: Vec<String>,
        pub content: String,
    }

    static DB_PATH: &'static str = ".test-db";
    
    #[test]
    fn test() {
        System::run(|| {
            let _ = remove_dir_all(DB_PATH);

            let storage = Storage::new(DB_PATH).unwrap();
            
            let addr = storage.start(3);
            let addr1 = addr.clone();
            let addr2 = addr.clone();
            let addr3 = addr.clone();
            
            spawn(
                addr.send(
                    Insert::<_, BlogPost>("blog", json_val!({
                        "title": "Absurd",
                        "tags": ["absurd", "psychology"],
                        "content": "Still nothing..."
                    }))
                ).and_then(move |res| {
                    assert_eq!(res.unwrap(), 1);
                    
                    addr1.send(Insert::<_, BlogPost>("blog", json_val!({
                        "title": "Lorem ipsum",
                        "tags": ["lorem", "ipsum"],
                        "content": "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."
                    })))
                }).and_then(move |res| {
                    assert_eq!(res.unwrap(), 2);

                    addr3.send(EnsureIndex("blog", "tags", IndexKind::Duplicate, KeyType::String))
                }).and_then(move |res| {
                    assert!(res.is_ok());
                    
                    addr2.send(Find::<_, BlogPost>("blog",
                                    json_val!({ "tags": { "$eq": "psychology" } }),
                                    json_val!("$asc")))
                }).map(|res| {
                    let mut docs = res.unwrap();
                    assert_eq!(docs.size_hint(), (1, Some(1)));
                    let doc = docs.next().unwrap().unwrap();
                    let doc_data: BlogPost = json_val!({
                        "title": "Absurd",
                        "tags": ["absurd", "psychology"],
                        "content": "Still nothing..."
                    });
                    assert_eq!(doc.get_data(), &doc_data);
                    assert!(docs.next().is_none());
                    
                    System::current().stop();
                }).map_err(|_| ())
            );
        });
    }
}
