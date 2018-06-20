# Plant Lifecycle

## Prior Art & Inspiration

### Life

* [Dormancy](https://en.wikipedia.org/wiki/Dormancy) (Wikipedia)
* [Abscission](https://en.wikipedia.org/wiki/Abscission) process for [Deciduous plants](#dormancy) (Wikipedia)

#### Plant Growth

* [Seedling](https://en.wikipedia.org/wiki/Seedling) (Wikipedia)
* [Plant reproduction](https://en.wikipedia.org/wiki/Plant_reproduction) (Wikipedia)
* [Secondary growth](https://en.wikipedia.org/wiki/Secondary_growth) (Wikipedia)

### Technology

* [Simulated growth of plants](https://en.wikipedia.org/wiki/Simulated_growth_of_plants) (Wikipedia)
  * Includes references to many extant software solutions to model plant growth
* [L-system branching](http://www.mizuno.org/applet/branching/)
* [/r/proceduralgeneration](https://www.reddit.com/r/proceduralgeneration/search?q=trees&restrict_sr=on) - Search for "trees"
* [TensorFlow Rust language bindings](https://github.com/tensorflow/rust)
* [Algorithmic Botany](http://algorithmicbotany.org/papers/) papers (University of Calgary)

## Philosophy

### Growth

#### Stages of Growth

1. Seed
2. Germination
3. Seedling
4. [Adult Lifecycle](#adult-lifecycle)

**A plant may [die](#death) during any of these stages.**

* Each stage of growth is distinct

> First-year seedlings typically have high mortality rates, drought being the principal cause, with roots having been unable to develop enough to maintain contact with soil sufficiently moist to prevent the development of lethal seedling water stress.
> 
> ...
>
> Seedlings are particularly vulnerable to attack by pests and diseases and can consequently experience high mortality rates. Pests and diseases which are especially damaging to seedlings include [damping off](https://en.wikipedia.org/wiki/Damping_off), [cutworms](https://en.wikipedia.org/wiki/Cutworm), [slugs](https://en.wikipedia.org/wiki/Slugs) and [snails](https://en.wikipedia.org/wiki/Snails).

([Seedling](https://en.wikipedia.org/wiki/Seedling) article on Wikipedia)

##### Adult Lifecycle

1. Growth (Primary and [secondary](https://en.wikipedia.org/wiki/Secondary_growth))
2. [Reproduction](#reproduction)
3. [Abscission](#dormancy) (Annual Dormancy)
  * **Only in deciduous plants**

#### Reproduction

Plants evolve and reproduce

Cycle of reproduction for flowering plants:
1. Seed
2. Germination
3. Growth
4. Flowering
5. Pollination
6. Seed Spreading (via Environment and Fauna, including human intervention)

See the [Plant reproduction](https://en.wikipedia.org/wiki/Plant_reproduction) article on Wikipedia.

##### Dormancy

Deciduous plants start becoming dormant during the Fall season and remain dormant through the Winter season.

See the [Dormancy](https://en.wikipedia.org/wiki/Dormancy) article on Wikipedia.

#### Death

##### Very Unhealthy Plants

Plants that have been [unhealthy](../health) for a long time start dying.

##### Dead Plants

A dead plant is still extant but may cause problems for the city; dry wood may start fires if struck by lightning, dead trees may fall on structures, and other disasters.

## Decisions

## Implementation Philosophy

* Structures
  * Plant
  * 

* "is-a" Traits
  * Seed
  * Deciduous (All without are evergreen)
  * Flower

See [Plant Seeds (Species & Growth Models)](../seeds) for Plant, Species ([Phyla](https://en.wikipedia.org/wiki/Phylum)), and Growth Model structures/traits

## Implementation Decisions
