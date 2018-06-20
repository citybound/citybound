# Vegetation

Deciduous and evergreen plants exhibit different lifecycles. Some plants die in
a drought. Agriculture is built off the backs of plants. Plants in a rainforest
are super-green and lush. There's a myriad of diversity, both natural and
artificial, in the plant kingdom. Citybound should reflect that fact.

## Prior Art & Inspiration

### Nature

* [Biome Classifications](https://en.wikipedia.org/wiki/Biome#Classifications) (Wikipedia)
* [Tree](https://en.wikipedia.org/wiki/Tree) (Wikipedia)
* [Shrub](https://en.wikipedia.org/wiki/Shrub) (Wikipedia)

#### Taxonomies of Life

* [Taxonomies of Life](https://en.wikipedia.org/wiki/Taxonomy_(biology)) (Wikipedia)
* [Plantae kingdom](https://en.wikipedia.org/wiki/Plant) (Wikipedia)
* [Phyla](https://en.wikipedia.org/wiki/Phylum) (Wikipedia) - Taxonomic rank below Kingdom
* [Plant divisions](https://en.wikipedia.org/wiki/Category:Plant_divisions) article category (Wikipedia)
* [Pinophyta](https://en.wikipedia.org/wiki/Pinophyta) plant phylum (Wikipedia) - i.e. Conifers, Pine trees
* [Angiosperms](https://en.wikipedia.org/wiki/Flowering_plant) clade (Wikipedia) - i.e. flowering plants

## Philosophy

* Deciduous and evergreen plants exhibit different lifecycles
* Some plants die in a drought
* Agriculture is built off the backs of plants
* Plants in a rainforest are super-green and lush
* Citybound should reflect the myriad of diversity in the plant kingdom, both natural and artificial

* When a new game starts the city will have been pre-planted with vegetation
* The player will build on, over, and around this extant vegetation
* Vegetation may be planted or uprooted as the player zones their city
* The foliage of a city literally breathes, evolves, and grows

### Plant Types

* Trees
  * Evergreen Conifers/Pines
  * Deciduous
* Bushes/Shrubs
* Cacti
* Crops
  
#### Traits

* Deciduous
* Flowering
* Annual (e.g. Tulips)
  * Crops

### Vegetation as an Emergent System `(emergence)`

* From a city's ~~[Climate]()~~ emerges its ~~[Biome]()~~
* From the current ~~[Season]()~~ and ~~[Weather]()~~ emerges [Plant Health]() and the visual representation of vegetation

#### ~~[Agriculture]()~~/Crops are Vegetation

* A crop's health is a function of its plants' health
  * So, for example, if a particular Summer season is so hot as to cause a drought and the city's water supply is taxed then a crop's health may degrade due to a lack of water
  * A crop can fail which lowers the agricultural supply of crops to the city's industry
  * A lowered supply of enough crops could cause food shortages

### Questions

* Should a city exist in a single climate?
* Should a city exist in a single or multiple biomes?

* What other types of plants?

## Decisions

* All vegetation is procedurally generated

## Implementation Philosophy

* Seems highly parallelizable

**Questions**

Perhaps we can use a Machine Learning/Tensor-based model? 

## Implementation Decisions

## Parts

* [Plant Health](./health)
* [Plant Lifecycle](./lifecycle)
* [Plant Seeds](./seeds) (Species & Growth Models)
* [Interaction](./interaction) (Planting & Uprooting Vegetation)
* [Rendering](./rendering)
