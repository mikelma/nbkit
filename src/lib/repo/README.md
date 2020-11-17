# Nebula Repository

## Structure of the repository

The root directory of the repository is always `repo`. Inside `repo`, there is a directory for each supported architecture (with the same name as the architecture), for example `x86_64` or `arm`. Inside an `{arch}`, there is a single index file (`index.toml`) and two directories, `bin` and `src`. 

The `index.toml` file contains information about all the packages for the supported architecture. 
`src` and `bin` are symetrical, in the sense that if a package exists inside one of this directories, the same package also exists inside the other directory.

`src` contains the source files for the packages, that contains (at least) a `template` script (executed to build the sources) and all pathces, configurations... needed to build the package and cannot be downloaded by `template` in execution time. 
In the other hand, inside `bin` the compressed packages and its SHA2556 hashes are stored. 

```
repo/
|
|-- {arch}/
|      |_____ index.toml
|      |_____ bin/
|      |       |_____ core/
|      |                |_____ foo/
|      |                        |_____ foo.tar.xz
|      |                        |_____ foo.sha256
|      |_____ src/
|              |_____ core/
|                       |_____ foo/
|                               |_____ template
```
