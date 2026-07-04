# Design for code updates in Meerkat

## Syntax

The syntax for code updates should distinguish updating existing definitions
from introducing new definitions.  One possibility would look like this
(example borrowed from the Requirements wiki page):

```meerkat
update s1 {
  update def y = x + 5;
}
```meerkat

## Service classes

Web clients will probably want to instantiate services defined on the server
that they connect to.  When we update the definition of the service we'd like
all instances of that service to be updated.  That suggests the idea of a
service class.  A service class is a template for instantiating services; the
most common use case is web clients, but there could be others.  Possible
syntax for service class definition and instantiation:

```meerkat
service class MyServiceClass {
  ...
}

service myService = MyServiceClass;
```meerkat

## Typechecking

A starting point for type checking code updates is found in
[Domingues and Seco 2015](https://docentes.fct.unl.pt/jrcs/files/rebls15.pdf).
The type checker knows about all the dependencies of a service, so if a
code update changes the interface of a service, the type checker will verify
that client code is not broken (and/or an update to the client code is
packaged with the service code update).

It should be possible to type check a code update against code in one or more
`.mkt` files, or against one or more running services.  In the latter case,
the code update is guaranteed to succeed unless the code is changed by some
other update in between.

## Coordination

A design choice we might consider is how to manage code updates in a Meerkat
distributed virtual machine.  First, a simple approach for specifying updates.
Services and service classes live where they were originally defined.  In the
REPL at that node, `update` statements can be executed to update them.  If the
node imports services from other nodes, it can also execute `update` statements
on those imported services.

When Meerkat supports an import statement in the REPL accepting a URL
(Issue #95), then we can easily import any service we want to update.

When an update is executed, we need to acquire a write-lock on any service
being deleted and on any members being deleted or modified.  That write-lock
will prevent any transactions from reading or writing to the locked members.
The type checker is run after locks are acquired but before applying the code
update, and if there are type errors, the locks are released, the update is
canceled, and error messages are reported to the developer.

## Errors due to code update races

A code update will always preserve the code in a working state.  However, there
could be races between executing some action and code updates.  Whichever
acquires the appropriate locks will run.  If the code update runs first, some
triggered actions could be invalidated.  We need to design a mechanism to report
an error to the user in such cases, and/or for the programmer to catch any
errors that are triggered by the update and run some other compensating action.
