//! Port of `__tests__/fixtures/stars-stuff.ts` and the `getSubgraphs()` loaders of
//! `__tests__/fixtures/{dgs,huge-schema}/index.ts` (which read the `.graphql` files in place).

use std::path::Path;

use federation_composition_rs::types::ServiceDefinition;

use super::testkit::service;
use super::utils::graphql;

/// The object `createStarsStuff()` returns.
pub struct StarsStuff {
    pub inventory: ServiceDefinition,
    pub reviews: ServiceDefinition,
    pub pandas: ServiceDefinition,
    pub products: ServiceDefinition,
    pub users: ServiceDefinition,
}

/// `createStarsStuff()`.
pub fn create_stars_stuff() -> StarsStuff {
    StarsStuff {
        inventory: service(
            "inventory",
            graphql(
                r#"
      extend schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.1"
          import: ["@key", "@shareable", "@external", "@requires"]
        )

      type Product implements ProductItf @key(fields: "id") {
        id: ID!
        dimensions: ProductDimension @external
        delivery(zip: String): DeliveryEstimates
          @requires(fields: "dimensions { size weight }")
      }

      type ProductDimension @shareable {
        size: String
        weight: Float
      }

      type DeliveryEstimates {
        estimatedDelivery: String
        fastestDelivery: String
      }

      interface ProductItf {
        id: ID!
        dimensions: ProductDimension
        delivery(zip: String): DeliveryEstimates
      }

      enum ShippingClass {
        STANDARD
        EXPRESS
        OVERNIGHT
      }
    "#,
            ),
        ),
        reviews: service(
            "reviews",
            graphql(
                r#"
      schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.0"
          import: ["@key", "@override", "@shareable"]
        ) {
        query: Query
      }

      type Product implements ProductItf @key(fields: "id") {
        id: ID!
        reviewsCount: Int!
        reviewsScore: Float! @shareable @override(from: "products")
        reviews: [Review!]!
      }

      interface ProductItf {
        id: ID!
        reviewsCount: Int!
        reviewsScore: Float!
        reviews: [Review!]!
      }

      type Query {
        review(id: Int!): Review
      }

      type Review {
        id: Int!
        body: String!
      }
    "#,
            ),
        ),
        pandas: service(
            "pandas",
            graphql(
                r#"
      directive @tag(name: String!) repeatable on FIELD_DEFINITION

      type Query {
        allPandas: [Panda]
        panda(name: ID!): Panda
      }

      type Panda {
        name: ID!
        favoriteFood: String @tag(name: "nom-nom-nom")
      }
    "#,
            ),
        ),
        products: service(
            "products",
            graphql(
                r#"
      extend schema
        @link(
          url: "https://specs.apollo.dev/federation/v2.1"
          import: [
            "@key"
            "@shareable"
            "@tag"
            "@inaccessible"
            "@composeDirective"
          ]
        )
        @link(
          url: "https://myspecs.dev/myDirective/v1.0"
          import: ["@myDirective", { name: "@anotherDirective", as: "@hello" }]
        )
        @composeDirective(name: "@myDirective")
        @composeDirective(name: "@hello")

      directive @myDirective(a: String!) on FIELD_DEFINITION
      directive @hello on FIELD_DEFINITION

      type Query {
        allProducts: [ProductItf]
        product(id: ID!): ProductItf
      }

      interface ProductItf implements SkuItf {
        id: ID!
        sku: String
        name: String
        package: String
        variation: ProductVariation
        dimensions: ProductDimension
        createdBy: User
        hidden: String @inaccessible
        oldField: String @deprecated(reason: "refactored out")
      }

      interface SkuItf {
        sku: String
      }

      type Product implements ProductItf & SkuItf
        @key(fields: "id")
        @key(fields: "sku package")
        @key(fields: "sku variation { id }") {
        id: ID! @tag(name: "hi-from-products")
        sku: String
        name: String @hello
        package: String
        variation: ProductVariation
        dimensions: ProductDimension
        createdBy: User
        hidden: String
        reviewsScore: Float!
        oldField: String
      }
      enum ShippingClass {
        STANDARD
        EXPRESS
      }
      type ProductVariation {
        id: ID!
        name: String
      }
      type ProductDimension @shareable {
        size: String
        weight: Float
      }
      type User @key(fields: "email") {
        email: ID!
        totalProductsCreated: Int @shareable
      }
    "#,
            ),
        ),
        users: service(
            "users",
            graphql(
                r#"
      directive @tag(name: String!) repeatable on FIELD_DEFINITION | OBJECT

      type User @key(fields: "email") {
        email: ID! @tag(name: "test-from-users")
        name: String
        totalProductsCreated: Int
      }
    "#,
            ),
        ),
    }
}

/// `getSubgraphs()` of `__tests__/fixtures/<dir>/index.ts`: every `.graphql` file of the
/// directory, named after the file without `.graphql`, in `readdir` order (sorted by name).
fn get_subgraphs(dir: &str) -> Vec<ServiceDefinition> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../__tests__/fixtures")
        .join(dir);
    let mut files: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .into_string()
                .expect("a UTF-8 file name")
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .filter(|file| file.ends_with(".graphql"))
        .map(|file| {
            let schema = std::fs::read_to_string(dir.join(&file)).unwrap_or_else(|e| panic!("reading {file}: {e}"));
            service(&file.replacen(".graphql", "", 1), graphql(&schema))
        })
        .collect()
}

/// `getSubgraphs()` of `__tests__/fixtures/dgs/index.ts`.
pub fn get_subgraphs_of_dgs() -> Vec<ServiceDefinition> {
    get_subgraphs("dgs")
}

/// `getSubgraphs()` of `__tests__/fixtures/huge-schema/index.ts`.
pub fn get_subgraphs_of_huge_schema() -> Vec<ServiceDefinition> {
    get_subgraphs("huge-schema")
}
