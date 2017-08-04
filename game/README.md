# Citybound Design Doc

This file gives a bird's-eye overview of what Citybound should become and why, how it will be implemented and all the different things that inspire it.

*Words in ALLCAPS refer to reocurring central themes of Citybound*

## Design Philosophy

* A game about the beautiful, emergent complexity of real cities (EMERGENT COMPLEXITY)
* A joyful balance between authoritative control and organic development (CITY = ORGANISM)
* The challenge of understanding interdependent systems that form a whole (INTERDEPENDENT SYSTEMS) 
* Interaction through powerful but simple tools with haptic qualities (PAINTING A WORLD)
* Clear means to see patterns of behavior and consequences of one’s actions - in the small and in the large (VISIBILITY)
* Planning & collaboration is the way humans work to achieve great things (PLANNING & COLLABORATION)

## Design Prior Art & Inspiration

* [SimCity Series](https://en.wikipedia.org/wiki/SimCity)
* [A-Train (Take the A-Train III)](https://en.wikipedia.org/wiki/A-Train#A-Train_III)
* [Rollercoaster Tycoon Series](https://en.wikipedia.org/wiki/RollerCoaster_Tycoon)
* [Factorio](https://en.wikipedia.org/wiki/Factorio)

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

* [Engine](../engine/README.md) & [Simulation Core](core/README.md) (30% alpha)
* [Transport](transport/README.md) (20% alpha)
* [Economy](economy/README.md) (5% alpha)
* [Construction](construction/README.md) (20% alpha)
* [Environment](environment/README.md) (0% alpha)

## Gameplay Aspects

* Planning
  * [Transport Infrastructure](transport/planning/README.md) (20% alpha)
  * [Utility Infrastructure](utilities/README.md) (0% alpha)
  * [Zoning](zoning/README.md) (0% alpha)
  * [Services](services/README.md) (0% alpha)
  * [Terraforming](environment/terraforming/README.md) (0% alpha)
* Finances
  * [Projects & Grants](projects/README.md) (0% alpha)
  * [Budgeting](finances/README.md) (0% alpha)
  * [Taxing](finances/README.md) (0% alpha)
* Inspection
  * [Statistics & Maps](inspection/stats/README.md) (0% alpha)
  * [Individual Households](inspection/households/README.md) (0% alpha)
  * [Stories](inspection/stories/README.md) (0% alpha)
