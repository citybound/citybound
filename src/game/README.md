# Citybound

*Words in ALLCAPS refer to reocurring central ideas behind Citybound*

## Design Philosophy

* A game about the beautiful, emergent complexity of real cities (EMERGENT COMPLEXITY)
* A joyful balance between authoritative control and organic development (CITY = ORGANISM)
* The challenge of understanding interdependent systems that form a whole (INTERDEPENDENT SYSTEMS) 
* Interaction through powerful but simple tools with haptic qualities (PAINTING A WORLD)
* Clear means to see patterns of behavior and consequences of one’s actions - in the small and in the large (VISIBILITY)
* Planning & collaboration is the way humans work to achieve great things (PLANNING & COLLABORATION)

## Design Decisions

* Play in large continuous regions with several million inhabitants (VASTNESS)
* Overlayed time-scales: Daily and yearly activities happen at a similar pace (MULTI-SCALE)
* Persistent and unique individuals (people & businesses) (DIVERSITY)

## Implementation Philosophy

* Develop systems from first principles, with complex behaviors emerging from simple microscopic interactions (MICROSCOPIC-UP)
* Solve problems generally, do not restrict thinking in specialized problem instances (ACTUAL PROBLEM)
* Be brave to do things in new ways, where necessary (NEW & DIFFERENT)

## Implementation Decisions

* Use Rust as the main programming language for high performance and a strong type system to rely on
* Use an actor-based model for isolation, dynamic dispatch and simple parallelization and networking

## Parts

*% indicate "felt implementation completion"*

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
