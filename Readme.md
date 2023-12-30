### Overview
Buckets is a tool for game asset management. It version controls game assets and the process of moving
an asset from one stage to another. Every stage is represented by a bucket which contains all the resources
to create a game asset in that stage of the production pipeline. The workflow is represented by linking buckets together, 
so that the output of one bucket is the input of another bucket. 

To make sure everything is in the bucket to do the work for that stage you can set 'expectations', simple
rules to check if everything is correct. Once you finish your work in a bucket you can finalize it, which
will automatically move the output of that bucket to the next bucket in the workflow.

### Example
Let's say you want to create a 3D model for a game. The model needs concept art and textures.
For this you can create two buckets with expectations. The first bucket contains concept art and the second bucket 
the contains the 3D model. The first bucket has two expectations:
1. The bucket has concept art
2. The concept art is approved by the art director

The second bucket has two expectations:
1. There is concept art for the model
1. The bucket has a 3D model
1. There are textures for the model

Both buckets are linked together so that the output of the first bucket is the input of the second bucket.

Once the concept art is ready and approved, you can 'finalize' the bucket. The second bucket will automatically receive the concept art,
which will satisfy the first expectation. Now you can create the 3D model and textures. Once finished and all expectations are met 
you can finalize the bucket, and it's ready for use in the game. 

Buckets are generally defined per person or team who create the content. So if you are a 3D artist you will have a bucket for 
your 3D models and textures and if you are a concept artist you will have a bucket for your concept art.

To make it possible to iterate over multiple versions you can give a version number to a finalized bucket. 
Meaning you can have multiple 'final' versions of you assets. This is useful if you want to keep track of the
changes you made to an asset which have dependencies on other assets. For example, if you change the concept art
of a character, you will also have to change the 3D model and textures. By giving a version number you will know 
which version of the 3D model and textures are based on which version of the concept art.

### Functionality
- Has buckets for every stage
- Every bucket can set expectations
- Buckets can be linked into a workflow

### Commands
`bucket init`
Initialize bucket repository

#### Buckets
`bucket create [name]`
Create a bucket for content

`bucket commit [message]`
Set the version of a bucket and store its content

`bucket finalize [version]`
Finalize a bucket and store its content

`bucket list`
Lists all buckets in a repository

`bucket history`
List all commits in a bucket

`bucket status`
Show which files have changed since the last commit

`bucket revert all`
Discards all changes and restores last commit

`bucket revert [file]`
Discards changes of a specific file and restores the file as it was in the
last commit

`bucket rollback [file] [commit id]`
Replaces a committed file in the bucket to the version found in the bucket with the specified commit id

`bucket rollback all [commit id]`
Replaces all committed files in the bucket with the versions found in the bucket with the specified commit id

`bucket stash`
Temporarily stashes the current version so you can retrieve another version

`bucket stash restore`
Restores stash

#### Rules and expectations
`bucket expect bucket [name]`
Expect the existence of a bucket with specified name

`bucket expect set file [type] [bucket directory]`
Set what file to expect in bucket

`bucket check`
Check if all expectations are met. If not, print what is missing.

`bucket link [from bucket directory] [to bucket directory]`
Create a one way link between two buckets


