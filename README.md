# Vectra
Simple local vector engine

## commands
Create versdb

`vectra create test d=5 t=f32`

Push ver to versdb

`vectra insert test v1 v2 v3 v4 v5 name=xxx...`

Query in versdb

`vectra find test v1 v2 v3 v4 v5 limit=10 f=l1`

## Vector distance calculation method
|f|type|
|---|---|
|eu|Euclidean Distance|
|l1|Manhattan Distance|
|cd|Chebyshev Distance|
|md|Minkowski Distance|
|cs|Cosine Similarity|
|js|Jaccard Similarity|
|md|Mahalanobis Distance|
|hd|Hamming Distance|