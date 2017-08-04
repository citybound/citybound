# Citybound Design Doc

This document gives a bird's-eye overview of what Citybound should become and why, how it will be implemented and all the different things that inspire it. It links to more in-depth documents about all the different aspects of Citybound

*Words in `(typewriter)` refer to reocurring central themes of Citybound*

## Design Philosophy

* A game about the beautiful, emergent complexity of real cities `(emergent complexity)`
* A joyful balance between authoritative control and organic development `(city = organism)`
* The challenge of understanding interdependent systems that form a whole `(interdependent systems) `
* Interaction through powerful but simple tools with haptic qualities `(painting a world)`
* Clear means to see patterns of behavior and consequences of one’s actions - in the small and in the large `(clarity)`
* Planning & collaboration is the way humans work to achieve great things `(planning)` `(collaboration)`

## Design Prior Art & Inspiration

* [SimCity Series](https://en.wikipedia.org/wiki/SimCity)
* [A-Train (Take the A-Train III)](https://en.wikipedia.org/wiki/A-Train#A-Train_III)
* [Rollercoaster Tycoon Series](https://en.wikipedia.org/wiki/RollerCoaster_Tycoon)
* [Factorio](https://en.wikipedia.org/wiki/Factorio)

## Design Decisions

* Play in large continuous regions with several million inhabitants `(vastness)` `(multi-scale)`
* Overlayed time-scales: Daily and yearly activities happen at a similar pace `(multi-scale)`
* Persistent and unique individuals (people & businesses) `(diverse individuals)`

## Implementation Philosophy

* Develop systems from first principles, with complex behaviors emerging from simple microscopic interactions `(emergence)`
* Solve problems generally, do not restrict thinking in specialized problem instances `(actual problem)`
* Be brave to do things in new ways, where necessary `(radicality)`

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
