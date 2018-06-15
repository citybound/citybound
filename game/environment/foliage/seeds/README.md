# Plant Seeds (Species & Growth Models)

## Prior Art & Inspiration

### Life

* [Phyla](https://en.wikipedia.org/wiki/Phylum) (Wikipedia)
  * Life Taxonomic rank below Kingdom
* [Plant divisions](https://en.wikipedia.org/wiki/Category:Plant_divisions) article category (Wikipedia)
* [Gymnosperm](https://en.wikipedia.org/wiki/Gymnosperm) (Wikipedia)
  * A plant with unenclosed, "naked" seeds

### Technology

* [Simulated growth of plants](https://en.wikipedia.org/wiki/Simulated_growth_of_plants) (Wikipedia)
  * Includes references to many extant software solutions to model plant growth
* [L-system branching](http://www.mizuno.org/applet/branching/)
  * [petgraph](https://github.com/bluss/petgraph) crate
    * Graph data structure library for Rust
  * [froggy](https://github.com/kvark/froggy) crate
    * Component Graph System experiment
* [/r/proceduralgeneration](https://www.reddit.com/r/proceduralgeneration/search?q=trees&restrict_sr=on)
  * Search for "trees"
* [IXORA](http://www.ixora.org/) (Private company)
  * Tensor-based graph computations
* [TensorFlow Rust language bindings](https://github.com/tensorflow/rust)
* [Algorithmic Botany](http://algorithmicbotany.org/papers/) papers (University of Calgary)

## Philosophy

### Trees
* Woody Trunks
* Woody Branches
* Green Branches
* Stems
* Leaves

See [Lifecycle of a Tree](https://www.woodlandtrust.org.uk/blog/2017/06/life-cycle-of-a-tree/)

#### Evergreens

Evergreen trees do not undergo [Abscission](../lifecycle#dormancy) (Dormancy) during their lifecycle

##### Conifers

Cone-bearing trees usually with pine needles.

##### Broadleaf

See [12 Broadleaf Evergreen Street Trees](https://www.portlandoregon.gov/parks/article/514100) for a list of broadleaf evergreen trees

### Bushes
* Green Branches
* Stems
* Leaves
* "Bushy-ness" factor (i.e. How voluptuous is this bush?)
* Shapes
  * Circular/Spherical
  * Ovular (Ovular-prism?)
  * Organic irregularities
  * Manicured
    * **These cost money from the owner! (public or private)**

#### Shrubs

Shrubs are bushes with Woody Branches and are usually larger than a Bush and smaller than a Tree. Shrubs also have thicker foliage than that of a Bush.

### Flowers
* Stems
* Leaves
* Flowers
* Flowering bushes
  * Flowering from stems growing on branches

### Questions

* Should we model most of the [Kingdom of Plantae](https://en.wikipedia.org/wiki/Plant)? 💥
* Can an organic bush become manicured?

## Decisions

* Graph-based L-system?
* Machine Learning, an [IXORA](http://www.ixora.org/)-esque tensor-based graph analog?

## Implementation Philosophy

**Questions**
* Should the foliage growth model use a tree/graph data structure?
  * If so, will we need new `CTree` and/or `CGraph` data structures in the [`compact`](https://github.com/citybound/citybound/tree/master/engine/compact) library?

### Parametric L-system Notation

* **Variables** : `A`, `B`, `Name`, `Seed`, `Branch`, etc.; Nodes in a plant's L-system tree
* **Constants** : `->`, `(`, `)`, `[`, `]`
    * `->` : Production
    * `(` : Denotes the beginning of a variable's parameter list
    * `)` : Close a parameter list
    * `[` : Denotes the beginning of a subtree for a preceding variable/variable-parameters-pair
      * The same is true for a variable/variable-parameters-pair preceded by a subtree
      * i.e. root tree structure and trunk/branching tree structure above roots
    * `]` : Close a subtree
* **Parameters** : `(` `paramsExpr` `)`
* **Parameters Expression** : `params`
    * | `params` `,` `params`
    * | `name` `=` `value`
* **Name** : `STRING`    
* **Value** : `value`
    * | `UINT`
    * | `UFLOAT`
    * | `STRING`
* **Production** : `A -> B`
    * `A` transforms into `B`
    * Meaning the tree on the left-hand-side is replaced with the right-hand-side

### Structures

#### Node thing

* Properties via `::std::ops::Deref` and `::std::ops::DerefMut` implementations? ([/r/rust Struct Composition vs Inheritance](https://www.reddit.com/r/rust/comments/51f60f/struct_composition_vs_inheritance/))
* `children: Vec[Node impls]` via 
* `parent: Node` reference?
* `impl std::iter::Iterator` (iterate over all children nodes, depth first traversal)
* `age: uint`
* `fn grow(&self, amount: uint = 1) -> Fate::Live | Fate::Die`
  * Default impl: `self.age += amount`

* A node's age is used in the [Plant Renderer](../rendering) as a measure of the node's length.
  * Higher the age, longer the plant node
  * Higher the age, larger the tapered radius of the node also?

* Plant
  * `impl Node`
  * `dicot: bool`
  
#### Plant Parts

All parts `impl Node`

* Seed
* Sprout
* Root
* Shoot
  * `Shoot` is a dicot if node's root `Plant:dicot` is `true`
* Trunk
* Branch
  * Root (`Branch-Root`)
  * Wood (`Branch-Wood`)
  * Green (`Branch-Green`)
* Stem
* Leaf
  * Every `Leaf` has a `Stem`, but a `Stem` can have more than one `Leaf`
  
**Questions**

* Is a `Trunk` simply a `Branch` with only one child?
  * **No**, a tree's trunk can have branches and still have a trunk continue skywards
        * So `Trunk` nodes can have child `Branch` and `Trunk` nodes, i.e. `Trunk [ Branch Trunk Branch ]`

### Traits

#### Plant Types

* Tree
* Bush
* Deciduous
  * All nodes without this trait are evergreen
* Flower

### Growth L-system

1. Seed
  * Begins as a lone seed, usually dried
2. Germination
  * `Seed -> Seed [ Sprout ]`
  * Germinates after planting with adequate water, food, and maximum distance to surface
3. Sprout
  * `1. Seed [ Sprout ] -> Root [ Shoot ]`
  * Becomes a rooted seedling when sprout breaks through soil's surface
  * If the plant is a dicot then the seedling will have two leaves, otherwise it's a whip
4. Seedling
  * `1. Root [ Shoot ] -> [ Root Root ] Branch-Root [ Branch-Green [ Stem [ Leaf ] Shoot Stem [ Leaf ] ] ]`
    * A whip; notice one leaf per `Stem`
  * `2. Root [ Shoot ] -> [ Root Root ] Branch-Root [ Branch-Green [ Stem [ Leaf Leaf ] Shoot Stem [ Leaf Leaf ] ] ]`
    * A dicot plant; notice two leaves per `Stem`
5. Large Seedling
  * `1. Seedling -> [ [ Root Root ] Branch-Root [ Root Root ] Branch-Root ] Branch-Root [ Branch-Green [ Branch-Green [ Stem [ Leaf ] Shoot Stem [ Leaf ] ] Branch-Green [ Stem [ Leaf ] Shoot Stem [ Leaf ] ] ] ]`
    * A whip; notice one leaf per `Stem`
  * `2. Seedling -> [ [ Root Root ] Branch-Root [ Root Root ] Branch-Root ] Branch-Root [ Branch-Green [ Branch-Green [ Stem [ Leaf Leaf ] Shoot Stem [ Leaf Leaf ] ] Branch-Green [ Stem [ Leaf Leaf ] Shoot Stem [ Leaf Leaf ] ] ] ]`
    * A dicot plant; notice two leaves per `Stem`
  * Becomes a Sapling
6. Sapling
  * **TODO:** L-System Productions
  * Branching root structure
  * Smooth bark on a woody trunk
  * "Symmetric" green branches
  * "Symmetric" stems with leaves surrounding green shoots
7. Primary growth
  * **TODO:** L-System Productions
  * More root branches
  * Rough bark trunk
  * Less rough bark branches
  * Green branches, stems, shoots, and leaves
8. Adult Lifecycle
  * **TODO:** L-System Productions
  * Reproductive leaf nodes

## Implementation Decisions
