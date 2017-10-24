# Citybound Design Doc

This document gives a bird's-eye overview of what Citybound should become and why, how it will be implemented and all the different things that inspire it. It links to more in-depth documents about all the different aspects of Citybound.

**This is a living and open document, find out [how to contribute your suggestions](../CONTRIBUTING.md).**

## Notation Conventions

* Words in `(typewriter)` refer to reocurring central themes of Citybound.

* Links in ~~strike-thorugh~~ link to a document that hasn't been written yet.

* Links followed by emoji question marks❓ refer to open questions with corresponding discussions/proposals

# Citybound

## Prior Art & Inspiration

* [SimCity Series](https://en.wikipedia.org/wiki/SimCity)
* [SimAnt](https://en.wikipedia.org/wiki/SimAnt)
* [A-Train (Take the A-Train III)](https://en.wikipedia.org/wiki/A-Train#A-Train_III)
* [Rollercoaster Tycoon Series](https://en.wikipedia.org/wiki/RollerCoaster_Tycoon)
* [Factorio](https://en.wikipedia.org/wiki/Factorio)

## Philosophy

* **A game about the beauty of emergent complexity**, on the familiar example of a city `(emergent complexity)`
* **The challenge of understanding interdependent systems that form a whole** `(interdependent systems) `
* A joyful balance between authoritative control and organic development `(city = organism)`
* Interaction through powerful but simple tools with haptic qualities `(painting a world)`
* Clear means to see patterns of behavior and consequences of one’s actions - in the small and in the large `(clarity)`
* Planning & collaboration is the way humans work to achieve great things `(planning)` `(collaboration)`

## Decisions

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

* ~~[Engine](../engine/README.md)~~
* Simulated World
  * [Simulation Core](core/README.md) (30% alpha)
  * [Transport](transport/README.md) (20% alpha)
  * [Economy & Household Behaviour](economy/README.md) (5% alpha)
  * ~~[Construction](construction/README.md)~~ (20% alpha)
  * ~~[Environment](environment/README.md)~~ (0% alpha)
* Player Interaction
  * [Planning](planning/README.md) (20% alpha)
  * [Governance](governance/README.md) (0% alpha)

## Gameplay & Skills

* "Building a city"
  * "Observation & Inspection"
    * ~~["Reading Statistics & Maps"](inspection/stats/README.md)~~ (0% alpha)
    * ~~["Inspecting Individual Households"](inspection/households/README.md)~~ (0% alpha)
    * ~~["Reading Stories"](inspection/stories/README.md)~~ (0% alpha)
  * "Planning"
    * ["Transport Infrastructure Planning"](transport/planning/README.md) (20% alpha)
    * ~~["Utility Infrastructure Planning"](utilities/README.md)~~ (0% alpha)
    * ~~["Zoning"](zoning/README.md)~~ (0% alpha)
    * ~~["Services Planning"](services/README.md)~~ (0% alpha)
    * ~~["Terraforming"](environment/terraforming/README.md)~~ (0% alpha)
  * "Execution & Finances"
    * ~~["Projects & Grants"](projects/README.md)~~ (0% alpha)
    * ~~["Budgeting"](finances/README.md)~~ (0% alpha)
    * ~~["Taxation"](finances/README.md)~~ (0% alpha)
