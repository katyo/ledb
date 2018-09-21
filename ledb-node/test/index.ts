import { deepStrictEqual as dse } from 'assert';
import { removeSync } from 'fs-extra';
import { Storage, Collection } from '../';

removeSync("test_db");

describe('storage', () => {
    const storage = new Storage("test_db/storage");
    
    it('get_info', () => {
        const info = storage.get_info();

        dse(typeof info, "object");
        dse(typeof info.map_size, "number");
    });

    it('get_stats', () => {
        const stats = storage.get_stats();

        dse(typeof stats, "object");
        dse(typeof stats.page_size, "number");
    });
});

describe('collection', () => {
    const storage = new Storage("test_db/collection");

    it('create', () => {
        dse(storage.has_collection("post"), false);
        dse(storage.get_collections(), []);
        
        const coll = storage.collection("post");

        dse(coll.constructor, Collection);
        dse(storage.has_collection("post"), true);
        dse(storage.get_collections(), ["post"]);
    });

    it('insert', () => {
        const coll = storage.collection("post");

        dse(coll.has(1), false);
        dse(coll.has(2), false);
        dse(coll.get(1), null);
        dse(coll.get(2), null);

        const doc1 = {title: "Foo", tag: ["Bar", "Baz"], timestamp: 1234567890};
        const id1 = coll.insert(doc1);

        dse(id1, 1);
        dse(coll.has(1), true);
        dse(coll.has(2), false);
        dse(coll.get(1), { $: 1, ...doc1 });
        dse(coll.get(2), null);

        const doc2 = {title: "Bar", tag: ["Foo", "Baz"], timestamp: 1234567899};
        const id2 = coll.insert(doc2);

        dse(id2, 2);
        dse(coll.has(1), true);
        dse(coll.has(2), true);
        dse(coll.get(1), { $: 1, ...doc1 });
        dse(coll.get(2), { $: 2, ...doc2 });

        const doc3 = {title: "Baz", tag: ["Bar", "Foo"], timestamp: 1234567819};
        const id3 = coll.insert(doc3);

        dse(id3, 3);

        const doc4 = {title: "Act", tag: ["Foo", "Eff"], timestamp: 1234567819};
        const id4 = coll.insert(doc4);

        dse(id4, 4);
    });

    it('ensure_index', () => {
        const coll = storage.collection("post");
        
        dse(coll.has_index("title"), false);
        dse(coll.has_index("tag"), false);
        dse(coll.has_index("timestamp"), false);
        dse(coll.get_indexes(), []);

        dse(coll.ensure_index("title", "uni", "string"), true);
        
        dse(coll.has_index("title"), true);
        dse(coll.has_index("tag"), false);
        dse(coll.has_index("timestamp"), false);
        dse(coll.get_indexes(), [["title", "uni", "string"]]);
        
        dse(coll.ensure_index("tag", "dup", "string"), true);
        
        dse(coll.has_index("title"), true);
        dse(coll.has_index("tag"), true);
        dse(coll.has_index("timestamp"), false);
        dse(coll.get_indexes(), [["title", "uni", "string"],
                                 ["tag", "dup", "string"]]);

        dse(coll.ensure_index("timestamp", "dup", "int"), true);
        
        dse(coll.has_index("title"), true);
        dse(coll.has_index("tag"), true);
        dse(coll.has_index("timestamp"), true);
        dse(coll.get_indexes(), [["title", "uni", "string"],
                                 ["tag", "dup", "string"],
                                 ["timestamp", "dup", "int"]]);
    });

    it('find', () => {
        const coll = storage.collection("post");

        dse(coll.find(null).count(), 4);
        dse(coll.find({title:{$eq:"Foo"}}).count(), 1);
        dse(coll.find({tag:{$eq:"Baz"}}).count(), 2);
        dse(coll.find({tag:{$eq:"Foo"}}).count(), 3);
        dse(coll.find({$or:[{title:{$eq:"Foo"}},{title:{$eq:"Bar"}}]}).count(), 2);
        dse(coll.find({$not:{title:{$eq:"Foo"}}}).count(), 3);
    });

    // TODO: more tests
});

describe('documents', () => {
    const storage = new Storage("test_db/collection");
    
    it('next', () => {
        const coll = storage.collection("post");
        let docs = coll.find(null);
        
        dse(docs.count(), 4);
        dse(docs.next(), {$: 1, title: "Foo", tag: ["Bar", "Baz"], timestamp: 1234567890});
        dse(docs.next(), {$: 2, title: "Bar", tag: ["Foo", "Baz"], timestamp: 1234567899});
        dse(docs.next(), {$: 3, title: "Baz", tag: ["Bar", "Foo"], timestamp: 1234567819});
        dse(docs.next(), {$: 4, title: "Act", tag: ["Foo", "Eff"], timestamp: 1234567819});
        dse(docs.next(), null);
    });
    
    it('skip', () => {
        const coll = storage.collection("post");

        dse(coll.find(null).skip(0).count(), 4);
        dse(coll.find(null).skip(1).count(), 3);
        dse(coll.find(null).skip(2).count(), 2);
        dse(coll.find(null).skip(3).count(), 1);
        dse(coll.find(null).skip(4).count(), 0);
        dse(coll.find(null).skip(5).count(), 0);
    });

    it('take', () => {
        const coll = storage.collection("post");

        dse(coll.find(null).take(0).count(), 0);
        dse(coll.find(null).take(1).count(), 1);
        dse(coll.find(null).take(2).count(), 2);
        dse(coll.find(null).take(3).count(), 3);
        dse(coll.find(null).take(4).count(), 4);
        dse(coll.find(null).take(5).count(), 4);
    });

    it('skip take', () => {
        const coll = storage.collection("post");

        dse(coll.find(null).skip(1).take(2).count(), 2);
        dse(coll.find(null).skip(2).take(3).count(), 2);
        dse(coll.find(null).skip(3).take(1).count(), 1);
        dse(coll.find(null).skip(3).take(2).count(), 1);
        dse(coll.find(null).take(3).skip(1).count(), 2);
        dse(coll.find(null).take(2).skip(1).count(), 1);
    });
});
