# Citybound

Citybound is an independently developed city building game, open source and funded though Patreon.
Find out more about Citybound [on its homepage](http://cityboundsim.com) and [on the wiki](https://github.com/aeickhoff/citybound/wiki).

* [Latest game releases](https://github.com/aeickhoff/citybound/releases)
* [Contributing & Development](CONTRIBUTING.md) ([Docs](http://citybound.github.io/citybound), [Gitter](https://gitter.im/citybound/Lobby))

## License

Citybound is licensed under AGPL (see [LICENSE.txt](LICENSE.txt)), for interest in commercial licenses please contact me.

# Design Documentation

This repository contains not only the code of Citybound, but also a detailed **Design Doc**, describing the philosophy and decisions taken for design and implementation.

**Just like the code, the Design Doc is open for contribution.** You can suggest changes by making a pull request, which can be discussed and refined before deciding if it will be taken fully, partially, or not - documenting the whole decision process publicly.

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
* Traffic (5%)
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
