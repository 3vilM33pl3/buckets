### Overview
Buckets is a tool for game asset management. It version controls game assets and the process of moving 
an asset from one stage to another. Every stage is represented by a bucket which contain all the resources 
to create a game asset in that stage of the production pipeline. Each bucket can be linked to a next bucket.

To make sure everything is in the bucket to do the work for that stage you can set 'expectations', simple 
rules which can be used to check if everything is correct. Once you finish your work in a bucket you can push 
it to the next bucket and check the expectations to see if everything is correct.

### Example
For example you could create a pipeline for texturing a 3D model. The first bucket contains concept art 
and a 3D model. The bucket has two expectations:
1. The bucket has a 3D model
2. There is concept art for the model

Once the concept art is ready and placed in the bucket you can create the 3D model.
Once finished you can check the expectations to see if your work is done. And when ready you push the model 
to the next bucket.

The next bucket is for texturing and has two expectations:
1. 3D model
2. Textures

Once the 3D model is ready you can texturize it. Once finished you can check the expectations to see if your work is done. 

Every step can be version controlled. A version always refers to all the content in
the bucket including the expectations. 

### Functionality
- Has buckets for every stage
- Every bucket can set expectations:
    - What to receive
    - What file format (Implicit what software is used )
    - File size
- Buckets can be linked into a workflow

### Commands
`bucket init`
Initialize bucket repository

`bucket create [name]`
Create a bucket for content

`bucket expect set file [type] [bucket directory]`
Set what file to expect in bucket

`bucket link [from bucket directory] [to bucket directory]`
Create a one way link between two buckets

`bucket commit [bucket] (major|minor|patch) [number]`
Set the version of a bucket and store its content

`bucket revert`
Deletes all changes and restores last commit

`bucket revert [file]`
Restores a specific file

`bucket version list`
List all stored versions

`bucket version get [version]`
Gets a specific version. Without version number shows a menu with versions to choose from.

`bucket stash`
Temporarily stashes the current version so you can retrieve another version

`bucket stash restore`
Restores stash


### File structure
#### Top level
`.buckets`
Contains general information. In a monorepo this is the top level directory

#### Per bucket container
`.b`
At the top of the container, contains general information:
* What version is currently visible in the bucket
* Where the location of the top of the (mono) repo is

`.b\.content`
Top level of all stored content.

`.b\.content\[hash]`
Every piece of content has it's own hash computed.

`.b\.content\[hash]\[file]`
Every file gets stored under it's own hash

`.b\1.2.3`
Versions get stored as the name of a directory

`.b\1.2.3\.rules`
Rules are versioned and get copied over when a new version is created

`.b\1.2.3\[directory]\[hash]`

`.\`
Content of a specific version. When content is set it gets copied into the top level bucket directory  from `.b\1.2.3\`. Hashes are replaced by the actual files.

