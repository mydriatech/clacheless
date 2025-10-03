# Clacheless

An extremely lightweight distributed in-memory cache for web application back-ends.

## Usage

### As a self-hosted service

Use the Helm chart and [OpenAPI](clacheless-api-rest/openapi.json) documentation
as a starting point for your integration.

Run the example Helm chart:

```
helm repo add mt-clacheless https://mydriatech.github.io/clacheless
helm repo update
helm upgrade --install --atomic --create-namespace --namespace clacheless-demo \
    clacheless mt-clacheless/clacheless
```

Verify the deployment using the bundled REST API CLI

```
kubectl -n clacheless-demo -i -t exec clacheless-0 -- clacheless-cli put awesome something
kubectl -n clacheless-demo -i -t exec clacheless-1 -- clacheless-cli get awesome
kubectl -n clacheless-demo -i -t exec clacheless-2 -- clacheless-cli get awesome
```

and you should see the value "something" being returned from the other instances.

### Kubernetes side-car in a `StatefulSet`

Use the Helm chart as a template and use Pod-internal REST API calls.

### Rust library to your `StatefulSet` app.

Add the crate as

```
[dependencies]
clacheless = { git = "https://github.com/mydriatech/clacheless.git", branch = "main" }
# Use a tag instead for production
#clacheless = { git = "https://github.com/mydriatech/clacheless.git", tag = "1.2.3" }
```

and follow the test [test_local_instance.rs](clacheless/tests/test_local_instance.rs)
for a simple example on how to get started.


## Security

The gRPC calls for interacting with the distributed cache are protected with a
simple token mechanism based on a shared secret between the Pods.

An attack that is able to eavesdrop on the gRPC-traffic between pods can
perform gRPC calls with full access using compromised tokens.
A compromised shared secret have similar implications.

## Caveats

When deploying the application as a `StatefulSet`, Kubernetes rolling upgrades
will prevent network traffic to reach the starting instance.

This means that you need to use at least 3 replicas so 2 of those can transfer
and keep any previous cached data while each of the nodes is being replaced.

## Name

A no clash cache -> Clacheless.

## License

[Apache License 2.0 with Free world makers exception 1.0.0](LICENSE-Apache-2.0-with-FWM-Exception-1.0.0)

The intent of this license to

* Allow makers, innovators, integrators and engineers to do what they do best without blockers.
* Give commercial and non-commercial entities in the free world a competitive advantage.
* Support a long-term sustainable business model where no "open core" or "community edition" is ever needed.

## Governance model

This projects uses the [Benevolent Dictator Governance Model](http://oss-watch.ac.uk/resources/benevolentdictatorgovernancemodel) (site only seem to support plain HTTP).

See also [Code of Conduct](CODE_OF_CONDUCT.md) and [Contributing](CONTRIBUTING.md).
