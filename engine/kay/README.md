# kay

kay is a...

- [X] High-performance actor system for Rust
- [X] suitable for simulating millions of entities in realtime
- [X] which only communicate using asynchronous messages

kay can be *transparently and trivially* scaled onto...

- [ ] multiple cores
- [X] multiple networked computers

It offers...

- [X] Actor `ID`s identifying
   - [X] Instances of an Actor Type
   - [X] Broadcasts to all instances of an Actor Type
   - [X] Specific-type-erased Actor Trait IDs
   - [X] Actor instances across networked computers
- [X] A `Recipient<Message>` trait in which a user implements message handling for each message type that an Actor can receive
- [X] `Swarm`s - collections of large numbers of instances of identical behaviour
   - [X] Compact and efficiently managed memory storage for dynamically-sized instance state, supplied by `chunked`
   - [X] Dispatch of messages to individual instances
   - [X] Very efficient broadcasting of a message to all instances
- [ ] Serialisation-free persistence, snapshotting and loading of actor and system state using memory-mapped files, implemented by `chunked`
- [ ] Future-like abstractions for awaiting and aggregating asynchronous responses from other actors
- [X] *"Essential"* message types that are handled even after a panic occurs in an Actor, allowing interactive inspection of the whole panicked system

It internally uses...

- [X] A message queue per Actor Type that is
   - [X] unbounded
   - [ ] multi-writer, single-reader
   - [ ] lock-free
- [X] The experimental `TypeId` feature, to tag message blobs with their type id for runtime message handling function dispatch
- [X] A [Slot Map](http://seanmiddleditch.com/data-structures-for-game-developers-the-slot-map/) in `Swarm`s to assign unique `ID`s to instances, while always keeping them in continous memory chunks. This makes iterating over them for broadcast messages very fast.

kay is inspired by Data-Oriented Game Development, Erlang and the original ideas behind Object-Orientedness. It is thus named after [Alan Kay](https://en.wikipedia.org/wiki/Alan_Kay).