
# Feature: Archive Resources

## Context

Currently the resources are loaded over a IRI it is possible to make
a iri point _into_ an archive e.g. the Java `jar` scheme does support
it so a program which wants to use e.g. whole templates from a 
archive could implement a resource loaded with a `archiev` prefix pointing
into an archive. 

## Problem 

If you point into an archive you would want to 1. open/load it 2.
get all resources you need for the given mail 3. close it. But the
per resource IRI architecture would require you to open/load it
for every Resource in it separately, clever caching can be done to some degree
(while it's open we do not need to open it another time etc.) but has
limitations. 

## Solution / Feature

let templates define a `resource_preload` and iri which
would point the the archive (but could also be used for other,
similar use cases, e.g. sharing a db connection).

Add a function to `ResourceLoaderComponent` like `fn use_preload(&self, preload: &IRI) {}` 
and `fn stop_using_preload(preload: &IRI) {}` which resource loader,
who can handle archives can use to preload/open an archive.

### Alternations

instead of a start/stop preload function have some scope based function,
potentially attached to the _Context_ instead of the `ResourceLoaderComponent` 