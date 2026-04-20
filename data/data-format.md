# How to read `gig4` data assets
Content in `gig4` is described to the game in the form of JSON assets. Each type of content (items, buildings, etc.) has its own JSON fields, which the game interprets as specified below.

## Terminology
Below are some definitions of terms that are used in the descriptions of content entries.

### Item list specifications

Item list specifications are used, among other things, to represent the following:
- The costs of [buildings](#buildings).
- The input and output items of [production rules](#production-rules).

An item list is specified in JSON as an array of objects (records) with these fields:

- `id`: The identifier of the item.
- `count`: The count of instances of that item.

## Buildings
Buildings' data are represented with the following fields:

- `id`: A string that serves as a unique identifier for the building.
- `name`: The building's name.
- `description`: The building's description.
- `baseCost`: An [item list](#item-list-specifications) specifying the cost to build the first building of that type. In order to construct the building, these resources must be moved to the construction site by workers.
- `costIncrease`: The multiplier by which the cost increases (multiplicatively) with each building of that type built.
- `firstFree`: Whether the first building of this type is free to build. Free buildings do not count towards cost scaling.
- `tierUpFrom`: The `id` of the building that this building is an upgrade to. Base-stage buildings do not have this field set, or otherwise have it set to `null`. If tiering up from an existing building, matching resources already in the existing building's inventory may be consumed in the process of tiering up.
- `footprint`: An object (also known as a record) that describes the building's footprint. If `tierUpFrom` is set to non-`null` and this field is not set, the `footprint` of the previous tier is inherited (recursively). The `footprint` record has the following fields:
    - `xSize`: The building's size in the $x$-dimension, in tile lengths.
    - `ySize`: The building's size in the $y$-dimension, in tile lengths.
- `inventorySize`: The size of the building's inventory in slots. Each slot can only hold one kind of item at a time. <!-- Buildings do not have a mass limit. (Mass is not currently implemented.)-->
- `productionRules`: An array of [production rule](#production-rules) IDs that this building supports.
- `productionSpeed`: A positive number that indicates how many times nominal speed the building produces at. If a building's `productionSpeed` is $2$ and it's producing under a rule that nominally takes $120$ seconds, that production instead takes $\frac{120}{2}=60$ seconds.

## Items
Items' data are represented by the following fields:

- `id`: A string that serves as a unique identifier for the item.
- `name`: The item's name.
- `category`: The `id` of the item's category. For workers, this is always the string `"worker"`.
- `description`: The item's description.
<!-- - (unimplemented) `mass`: The mass of one instance of the item, in kilograms. -->
- `stackLimit`: The item's stack size limit. Past this number, multiple slots are necessary to hold all of the items.
<!-- - `value`: The item's value. This is used in scoring. -->

### Workers
Workers share the following additional fields:
- `speed`: The worker's base speed in tile-lengths per second.
- `inventorySize`: The worker's inventory size limit, as a number of slots. Each slot can only hold one kind of item at a time.
<!-- - (unimplemented) `massSoftLimit`: The worker's soft mass limit in kilograms. Past this limit, the worker's speed is penalized by being multiplied by $\left(\frac{\mathrm{massLimit}}{\mathrm{heldMass}}\right)^{1.25}$. The worker's own mass is ignored in this calculation.
- (unimplemented) `massHardLimit`: The worker's hard mass limit in kilograms. Past this limit, the worker will fail to move. It will instead attempt to remove items from itself until it becomes able to move again. The worker's own mass is ignored in this calculation. -->

## Categories
Categories are some of the simplest `gig4` data asset entries, with only three string fields.

- `id`: A string that serves as a unique identifier for the category.
- `name`: The name of the category.
- `descrtiption`: A description of the category.

### Example category:

```json
{
    "id": "worker",
    "name": "Worker",
    "description": "Workers carry items around according to your requests. They're your only way of moving items between buildings, so you may need to deploy more of them as you scale your production."
}
```

## Production rules:
A simple production rule has the following fields:

- `id`: A string that serves as a unique identifier for this rule.
- `inputs`: A [list](#item-list-specifications) of input items for this rule.
- `outputs`: A [list](#item-list-specifications) of output items for this rule.
- `time`: The time the rule nominally takes. Higher-tier buildings may actually take less time to process this rule due to having inherent speed multipliers.
