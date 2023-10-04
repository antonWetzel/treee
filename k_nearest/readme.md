# k_nearest

- find the k-nearest neighbors for a list of points with arbitrary dimensions
- please write an issue for any bugs or feature requests

## Code example

```rust
const MAX_NEIGHBORS: usize = 16;
const MAX_DISTANCE: f32 = 1.0;

fn example(points: &[Point]) {
	let kd_tree = k_nearest::KDTree::<
		3,       // dimensions
		f32,     // type of a value for a dimension
		Point,   // point type
		Adapter, // adapter to allow any point type
		Metric,  // metric to calculate distances
	>::new(points);

	// iterate all points to calculate something
	for point in points.iter() {
		// space to insert result
		//   can be part of a larger memory location
		//   every entry contains the distance and the point index
		//   initial values are never read
		//   'neighbor.0' is the distance to the query point
		// 	 'neighbor.1' is the index in the list of points
		let mut neighbors = [(0.0, 0); MAX_NEIGHBORS];

		// calculate nearest points
		//   result at offset 0 is the original point
		//   result at offset 1 is the nearest neighbor in the search radius
		let count = kd_tree.k_nearest(point, &mut neighbors, MAX_DISTANCE);

		// create a subslice so neighbors only contains valid entries
		let neighbors = &neighbors[0..count];

		// start the calculations
		...
	}
}
```

## For a Point with values as members

```rust
struct Point {
	x: f32,
	y: f32,
	z: f32,
}

struct Adapter;
impl k_nearest::Adapter<3, f32, Point> for Adapter {

	fn get(point: &Point, dimension: usize) -> f32 {
		match dimension {
			0 => point.x,
			1 => point.y,
			2 => point.z,
			_ => unreachable!(),
		}
	}
}
```

## For a point with values as array

```rust
struct Point([f32; 3]);

struct Adapter;
impl k_nearest::Adapter<3, f32, Point> for Adapter {
	fn get(point: &Point, dimension: usize) -> f32 {
		point.0[dimension]
	}

	// get all values in one call
	//  default implementation calls Adapter::get N times
	fn get_all(point: &Point) -> [f32; 3] {
		point.0
	}
}
```

## Metric

```rust
// use euclidean distance
//   the resulting distances are all squared
type Metric = k_nearest::EuclideanDistanceSquared;

// or define own metric
struct ManhattenDistance;

impl<const N: usize> Metric<N, f32> for ManhattenDistance {
	fn distance(left: &[f32; N], right: &[f32; N]) -> f32 {
		(0..N)
			.map(|d| left[d] - right[d])
			.map(|v| v.abs())
			.fold(0.0, |sum, v| sum + v)
	}
	fn distance_plane(position: &[f32; N], plane: f32, dimension: usize) -> f32 {
		let diff = position[dimension] - plane;
		diff.abs()
	}
}
```
