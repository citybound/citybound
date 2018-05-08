# Car Microtraffic

## Philosophy & Decisions

* See [Transport](../README.md)

## Implementation Philosophy

* Keep car interactions as local as possible computationally

## Implementation Decisions

* **Cars themselves are not actors, one lane is the atomic actor,** it updates all the cars on it in one go
* Cars mostly move along a lane like a train, only on transfer lanes they can move sideways to merge between normal lanes
* Cars react to general obstacles, which are either the next car on their own lane, or cars on interacting lanes, whose position is mapped onto the current lane
    * Interactions are distinguished as parallel/opposing/merging
    * Interacting lanes after each update exchange their cars as obstacles for the other lane
* Use well-known 1D acceleration and breaking model and extend it to multi-lane behaviour
   * "Intelligent Driver Model", see [intelligent_acceleration.rs](./intelligent_acceleration.rs)
* Cars on transfer lanes have more complex behaviour, reacting to cars on their left & right lanes and other cars merging between them.
    * Based on the perceived risk/required breaking, merging cars can decide to abort their merge
* Also see [Road Lanes](../lane/README.md)
