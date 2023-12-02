# Glance

The quick media viewer.

# glance-lib

The core of glance is an index of media files for quick CRUD and search operations. [sqlite](https://docs.rs/sqlite/latest/sqlite/) is a natural choice for this index but other options could be explored. Potential columns in the index would include:


* `hash` - hash of the media file to easily check for existence
* `filepath` - location of the media on disk (could be made even more general by using url)
* `format` - media format (ie png, jpg, mp4, ...)
* `created` - media creation time
* `location` - media creation location in latitude an longitude
* `device` - device name used to capture the media
* `iso` - the iso setting used when producing the media (this is a useful proxy for if the media was created indoors or outdoors)

The operations on an `Index` include:

* `create` - create a new empty index
* `read_from_disk` - read an index from disk
* `write_to_disk` - write an index to disk
* `validate` - validate an index matches the data on disk
* `add_directory` - given a directory recursively searches for all media in the directory and adds them to an index
* `copy_from_directory` - given a directory copies media to a new directory and updates the index
    * useful for copying images off of a device
    * checks that the images do not already exist using the `hash`
    * should match the directory and file structure of the destination directory
* `search` - open search across media fields returning an iterator of matches
* `normalize_directory_structure` - has a variety of options for standardizing image naming on disk (ex. `/<year>/<nanosecond_timestamp>.<ext>`)

# glance-cli

A CLI wrapper over `glance-lib` to create and manage indexes. Its search functionality would output matching filepaths or produce a directory of symlinks.

# glance-ui
A UI wrapper over `glance-lib` with search functionality and media viewer.
