# Expectation system

## Expanded Explanation of the Expectation System

### What Expectations Are

- **Expectations** are rules or conditions that must be met before a bucket can be finalized.
- They can be **intrinsic** (things inside the bucket, like “has a texture file”) or **extrinsic** (dependencies on other buckets, like “Concept Art is approved” before 3D Model can finalize).
- Think of them like **checklists + dependency links**.

### How They Fit Into Workflow

1. **Defining Expectations (`buckets expect`)**
    - You attach one or more expectations to a bucket.
    - Intrinsic example:
    
    ```bash
    # intrinsic
    buckets expect "Has a mood board"
    ```
    
    - Extrinsic example:
    
    ```jsx
    # If command is executed in expecting bucket
    # buckets expect [expectation] from [fulfilling bucket]
    buckets expect "Concept art" from "Art bucket"
    
    # If command is executed outside expecting bucket
    # buckets expect [expectation] from [fulfilling bucket] in [expecting bucket]
    buckets expect "Concept art" from "Art bucket" in "3D bucket"
    ```
    
2. **Checking Progress (`buckets check`)**
    - At any time, you can check if a bucket’s expectations are satisfied.
    - Example output:
        
        ```
        Bucket: 3D Model
        [x] Has concept art for the model
        [ ] Has 3D model
        [ ] Has textures for the model
        
        ```
        
3. **Finalizing (`buckets finalize`)**
    - Once all expectations are satisfied, you can finalize the bucket.
    - Finalizing locks its output and makes it consumable for downstream buckets.
    - Example:
        
        ```bash
        buckets finalize [bucket name]
        ```
        

---

## Example Workflow (Expanded)

### Scenario: Game Asset Pipeline

- **Bucket 1: Concept Art**
    - Expectations:
        1. Mood board exists
        2. Concept sketches uploaded
        3. Art director approval attached (this is a personal digital signature)
    - Finalization: once all checked, Concept Art is frozen.
- **Bucket 2: 3D Model**
    - Expectations:
        1. Has finalized Concept Art (from bucket 1)
        2. A 3D model file (.blend, .fbx, etc.)
        3. A texture set (.png, .tga, etc.)
    - As soon as Concept Art is finalized, expectation (1) auto-completes.
    - Team then commits the model and textures.
- **Bucket 3: Animation**
    - Expectations:
        1. Finalized 3D model (from bucket 2)
        2. Rig controls defined
        3. Walk cycle completed

This creates a **chain of buckets**, where downstream work only progresses when upstream work is finalized. 

## Pebbles

On top of the expectation system, which is basically a graph connecting buckets to each other with expectations, there is a system of pebbles. Each time a new expectation gets created it generates a pebble, in the expecting bucket. This pebble than traverses down the expectation to the ‘other’ bucket. A pebble can only traverse an expectation once. So every buckets which has another bucket expecting something of it will have one or more pebbles in it.  On top of this, every time an expectation is created it also makes a copy of all the pebbles currently in the bucket and sends them together with the newly created one down the expectation to the ‘other’ bucket.  Now each time an expectation is met all the pebbles which traversed over said expectation are taken out of the bucket and put on the ‘resolved’ pile. 

Each pebble has a reference to their original bucket and a reference to the newly created pebble.