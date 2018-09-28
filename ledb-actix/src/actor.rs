use std::marker::PhantomData;
use std::path::Path;
use serde::{Serialize, de::DeserializeOwned};
use ledb::{Storage as LeStorage, Result as LeResult};
use actix::{Actor, Addr, Message, SyncContext, SyncArbiter, Handler};

use super::{
    Document, DocumentsIterator, Filter, Identifier, IndexKind, Info,
    KeyType, Modify, Order, Primary, Stats,
};

/// Storage actor
#[derive(Clone)]
pub struct Storage(LeStorage);

impl Storage {
    /// Instantiate new storage actor using path to the database in filesystem
    ///
    /// You can create multiple storage adapters using same path, actually all of them will use same storage instance.
    ///
    pub fn new<P: AsRef<Path>>(path: P) -> LeResult<Self> {
        Ok(Storage(LeStorage::new(path)?))
    }

    /// Start the actor with number of threads
    pub fn start(self, threads: usize) -> Addr<Self> {
        SyncArbiter::start(threads, move || self.clone())
    }
}

impl Actor for Storage {
    type Context = SyncContext<Self>;
}

/// Get database stats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetStats;

impl Message for GetStats {
    type Result = LeResult<Stats>;
}

impl Handler<GetStats> for Storage {
    type Result = <GetStats as Message>::Result;

    fn handle(&mut self, _: GetStats, _: &mut Self::Context) -> Self::Result {
        self.0.get_stats()
    }
}

/// Get database info
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetInfo;

impl Message for GetInfo {
    type Result = LeResult<Info>;
}

impl Handler<GetInfo> for Storage {
    type Result = <GetInfo as Message>::Result;

    fn handle(&mut self, _: GetInfo, _: &mut Self::Context) -> Self::Result {
        self.0.get_info()
    }
}

/// Get collections request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetCollections;

/// The list of collections
pub type ListCollections = Vec<String>;

impl Message for GetCollections {
    type Result = LeResult<ListCollections>;
}

impl Handler<GetCollections> for Storage {
    type Result = <GetCollections as Message>::Result;

    fn handle(&mut self, _: GetCollections, _: &mut Self::Context) -> Self::Result {
        self.0.get_collections()
    }
}

/// Ensure collection in storage
#[allow(non_snake_case)]
pub fn EnsureCollection<C: Into<Identifier>>(coll: C) -> EnsureCollectionMsg {
    EnsureCollectionMsg(coll.into())
}

/// Ensure collection in storage
///
/// *NOTE: Use `EnsureCollection` function instead*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnsureCollectionMsg(Identifier);

impl Message for EnsureCollectionMsg {
    type Result = LeResult<bool>;
}

impl Handler<EnsureCollectionMsg> for Storage {
    type Result = <EnsureCollectionMsg as Message>::Result;

    fn handle(&mut self, EnsureCollectionMsg(name): EnsureCollectionMsg, _: &mut Self::Context) -> Self::Result {
        Ok(if self.0.has_collection(&name)? {
            false
        } else {
            self.0.collection(name)?;
            true
        })
    }
}

/// Drop collection from storage
#[allow(non_snake_case)]
pub fn DropCollection<C: Into<Identifier>>(coll: C) -> DropCollectionMsg {
    DropCollectionMsg(coll.into())
}

/// Drop collection from storage
///
/// *NOTE: Use `DropCollection` function instead*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropCollectionMsg(Identifier);

impl Message for DropCollectionMsg {
    type Result = LeResult<bool>;
}

impl Handler<DropCollectionMsg> for Storage {
    type Result = <DropCollectionMsg as Message>::Result;

    fn handle(&mut self, DropCollectionMsg(collection): DropCollectionMsg, _: &mut Self::Context) -> Self::Result {
        self.0.drop_collection(collection)
    }
}

/// Get indexes of collection
#[allow(non_snake_case)]
pub fn GetIndexes<C: Into<Identifier>>(coll: C) -> GetIndexesMsg {
    GetIndexesMsg(coll.into())
}

/// The list of indexes
pub type ListIndexes = Vec<(Identifier, IndexKind, KeyType)>;

/// Get indexes of collection
///
/// *NOTE: Use `GetIndexes` function instead*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetIndexesMsg(Identifier);

impl Message for GetIndexesMsg {
    type Result = LeResult<ListIndexes>;
}

impl Handler<GetIndexesMsg> for Storage {
    type Result = <GetIndexesMsg as Message>::Result;

    fn handle(&mut self, GetIndexesMsg(collection): GetIndexesMsg, _: &mut Self::Context) -> Self::Result {
        self.0.collection(collection)?.get_indexes()
    }
}

/// Set indexes for collection
#[allow(non_snake_case)]
pub fn SetIndexes<C: Into<Identifier>, I: Into<ListIndexes>>(coll: C, indexes: I) -> SetIndexesMsg {
    SetIndexesMsg(coll.into(), indexes.into())
}

/// Set indexes for collection
///
/// *NOTE: Use `SetIndexes` function instead*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetIndexesMsg(Identifier, ListIndexes);

impl Message for SetIndexesMsg {
    type Result = LeResult<()>;
}

impl Handler<SetIndexesMsg> for Storage {
    type Result = <SetIndexesMsg as Message>::Result;

    fn handle(&mut self, SetIndexesMsg(collection, indexes): SetIndexesMsg, _: &mut Self::Context) -> Self::Result {
        self.0.collection(collection)?.set_indexes(&indexes)
    }
}

/// Ensure new index for collection
#[allow(non_snake_case)]
pub fn EnsureIndex<C: Into<Identifier>, F: Into<Identifier>>(coll: C, field: F, kind: IndexKind, key: KeyType) -> EnsureIndexMsg {
    EnsureIndexMsg(coll.into(), field.into(), kind, key)
}

/// Ensure new index for collection
///
/// *NOTE: Use `EnsureIndex` for creating message*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnsureIndexMsg(Identifier, Identifier, IndexKind, KeyType);

impl Message for EnsureIndexMsg {
    type Result = LeResult<bool>;
}

impl Handler<EnsureIndexMsg> for Storage {
    type Result = <EnsureIndexMsg as Message>::Result;

    fn handle(&mut self, EnsureIndexMsg(collection, field, kind, key): EnsureIndexMsg, _: &mut Self::Context) -> Self::Result {
        self.0.collection(collection)?.ensure_index(field, kind, key)
    }
}

/// Drop spicific index from collection
#[allow(non_snake_case)]
pub fn DropIndex<C: Into<Identifier>, F: Into<Identifier>>(coll: C, field: F) -> DropIndexMsg {
    DropIndexMsg(coll.into(), field.into())
}

/// Drop spicific index from collection
///
/// *NOTE: Use `DropIndex` for creating message*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropIndexMsg(Identifier, Identifier);

impl Message for DropIndexMsg {
    type Result = LeResult<bool>;
}

impl Handler<DropIndexMsg> for Storage {
    type Result = <DropIndexMsg as Message>::Result;

    fn handle(&mut self, DropIndexMsg(collection, field): DropIndexMsg, _: &mut Self::Context) -> Self::Result {
        self.0.collection(collection)?.drop_index(field)
    }
}

/// Insert new document into collection
#[allow(non_snake_case)]
pub fn Insert<C: Into<Identifier>, T: Serialize>(coll: C, data: T) -> InsertMsg<T> {
    InsertMsg(coll.into(), data)
}

/// Insert new document into collection
///
/// *NOTE: Use `Insert` for creating message*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertMsg<T>(Identifier, T);

impl<T: Serialize> Message for InsertMsg<T> {
    type Result = LeResult<Primary>;
}

impl<T: Serialize> Handler<InsertMsg<T>> for Storage {
    type Result = <InsertMsg<T> as Message>::Result;

    fn handle(&mut self, InsertMsg(collection, document): InsertMsg<T>, _: &mut Self::Context) -> Self::Result {
        self.0.collection(collection)?.insert(&document)
    }
}

/// Get the previously inserted document by primary key
#[allow(non_snake_case)]
pub fn Get<C: Into<Identifier>, T: DeserializeOwned>(coll: C, id: Primary) -> GetMsg<T> {
    GetMsg(coll.into(), id, PhantomData)
}

/// Get the previously inserted document by primary key
///
/// *NOTE: Use `Get` for creating message*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetMsg<T>(Identifier, Primary, PhantomData<T>);

impl<T: DeserializeOwned + 'static> Message for GetMsg<T> {
    type Result = LeResult<Option<Document<T>>>;
}

impl<T: DeserializeOwned + 'static> Handler<GetMsg<T>> for Storage {
    type Result = <GetMsg<T> as Message>::Result;

    fn handle(&mut self, GetMsg(collection, identifier, ..): GetMsg<T>, _: &mut Self::Context) -> Self::Result {
        self.0.collection(collection)?.get(identifier)
    }
}

/// Put new version of the previously inserted document
#[allow(non_snake_case)]
pub fn Put<C: Into<Identifier>, T: Serialize>(coll: C, data: Document<T>) -> PutMsg<T> {
    PutMsg(coll.into(), data)
}

/// Put new version of the previously inserted document
///
/// *NOTE: Use `Put` for creating message*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutMsg<T>(Identifier, Document<T>);

impl<T: Serialize> Message for PutMsg<T> {
    type Result = LeResult<()>;
}

impl<T: Serialize> Handler<PutMsg<T>> for Storage {
    type Result = <PutMsg<T> as Message>::Result;

    fn handle(&mut self, PutMsg(collection, document): PutMsg<T>, _: &mut Self::Context) -> Self::Result {
        self.0.collection(collection)?.put(&document)
    }
}

/// Delete the previously inserted document
#[allow(non_snake_case)]
pub fn Delete<C: Into<Identifier>>(coll: C, id: Primary) -> DeleteMsg {
    DeleteMsg(coll.into(), id)
}

/// Delete the previously inserted document
///
/// *NOTE: Use `Delete` for creating message*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteMsg(Identifier, Primary);

impl Message for DeleteMsg {
    type Result = LeResult<bool>;
}

impl Handler<DeleteMsg> for Storage {
    type Result = <DeleteMsg as Message>::Result;

    fn handle(&mut self, DeleteMsg(collection, id): DeleteMsg, _: &mut Self::Context) -> Self::Result {
        self.0.collection(collection)?.delete(id)
    }
}

/// Update documents using filter and modifier
#[allow(non_snake_case)]
pub fn Update<C: Into<Identifier>>(coll: C, filter: Option<Filter>, modify: Modify) -> UpdateMsg {
    UpdateMsg(coll.into(), filter, modify)
}

/// Update documents using filter and modifier
///
/// *NOTE: Use `Update` for creating message*
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateMsg(Identifier, Option<Filter>, Modify);

impl Message for UpdateMsg {
    type Result = LeResult<usize>;
}

impl Handler<UpdateMsg> for Storage {
    type Result = <UpdateMsg as Message>::Result;

    fn handle(&mut self, UpdateMsg(collection, filter, modify): UpdateMsg, _: &mut Self::Context) -> Self::Result {
        self.0.collection(collection)?.update(filter, modify)
    }
}

/// Remove documents using filter
#[allow(non_snake_case)]
pub fn Remove<C: Into<Identifier>>(coll: C, filter: Option<Filter>) -> RemoveMsg {
    RemoveMsg(coll.into(), filter)
}

/// Remove documents using filter
///
/// *NOTE: Use `Remove` for creating message*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveMsg(Identifier, Option<Filter>);

impl Message for RemoveMsg {
    type Result = LeResult<usize>;
}

impl Handler<RemoveMsg> for Storage {
    type Result = <RemoveMsg as Message>::Result;

    fn handle(&mut self, RemoveMsg(collection, filter): RemoveMsg, _: &mut Self::Context) -> Self::Result {
        self.0.collection(collection)?.remove(filter)
    }
}

/// Find documents using filter and ordering
#[allow(non_snake_case)]
pub fn Find<C: Into<Identifier>, T>(coll: C, filter: Option<Filter>, order: Order) -> FindMsg<T> {
    FindMsg(coll.into(), filter, order, PhantomData)
}

/// Find documents using filter and ordering
///
/// *NOTE: Use `Find` for creating message*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindMsg<T>(Identifier, Option<Filter>, Order, PhantomData<T>);

impl<T: DeserializeOwned + 'static> Message for FindMsg<T> {
    type Result = LeResult<DocumentsIterator<T>>;
}

impl<T: DeserializeOwned + 'static> Handler<FindMsg<T>> for Storage {
    type Result = <FindMsg<T> as Message>::Result;

    fn handle(&mut self, FindMsg(collection, filter, order, ..): FindMsg<T>, _: &mut Self::Context) -> Self::Result {
        self.0.collection(collection)?.find(filter, order)
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

    static DB_PATH: &'static str = "test_db";
    
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
