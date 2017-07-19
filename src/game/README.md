# Citybound

## Design Philosophy

* A game about the beautiful complexity of real cities
* A joyful balance between authoritative control and organic, serendipitous development
* Interaction through powerful but simple tools with haptic qualities
* Clear visual means to see patterns of behavior and consequences of one’s actions - in the small and in the large
* Collaboration is the way humans work to achieve great things

## Design Decisions

* Play in large continuous regions with several million inhabitants
* Overlayed time-scales: Daily and yearly activities happen at a similar pace (MULTI-SCALE)
* Persistent and unique individuals (people & businesses)

## Implementation Philosophy

* Develop systems from first principles, with complex behaviors emerging from simple microscopic interactions
* Never build lossy simplifications, rather rely on full simulation of individuals
* Solve problems generally, do not restrict thinking in specialized problem instances

## Implementation Decisions

* Use Rust as the main programming language for high performance and a strong type system to rely on
* Use an actor-based model for isolation, dynamic dispatch and simple parallelization and networking

## Parts

* Engine & Simulation Core (10%)
* [Traffic](lanes_and_cars/README.md) (5%)
* Economy (1%)
* Construction (1%)
* Environment

## Gameplay Aspects

* Planning
  * Transport Infrastructure (5%)
  * Zoning
  * Services
  * Terraforming
* Finances
  * Budgeting
  * Taxing
  * Grants
* Inspection
  * Individual Households
  * Statistics & Maps
  * Stories
