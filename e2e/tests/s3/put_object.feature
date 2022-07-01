Feature: PutObject

  Scenario: PutObject: regular file
    When `rms.jpg` is uploaded to `data` bucket as `dir/file`
    Then `rms.jpg` is stored as `/data/dir/file`

  Scenario: PutObject: symlink
    When `rms.jpg` is uploaded to `data` bucket as `dir/file`
    And `dir/link` symlink is created on `links` bucket pointing to `data/dir/file`
    Then `rms.jpg` is accessible via `links/dir/link`

  Scenario: PutObject: overwrite regular file
    Given `rms.jpg` is uploaded to `data` bucket as `a/b/file`
    When `ignucius.jpg` is uploaded to `data` bucket as `a/b/file`
    Then `ignucius.jpg` is stored as `data/a/b/file`

  Scenario: PutObject: overwrite symlink
    Given `rms.jpg` is uploaded to `data` bucket as `dir/file1`
    And `ignucius.jpg` is uploaded to `data` bucket as `dir/file2`
    And `link` symlink is created on `links` bucket pointing to `data/dir/file1`
    When `link` symlink is created on `links` bucket pointing to `data/dir/file2`
    Then `ignucius.jpg` is accessible via `links/link`

  Scenario: PubObject: invalid keys: root component
    When trying to upload files with the following keys:
      | /abc   |
      | /abc/d |
    Then `InvalidArgument` error is returned

  Scenario: PutObject: invalid keys: current/parent dir or empty component
    When trying to upload files with the following keys:
      | ./abc    |
      | abc/.    |
      | abc/./d  |
      | ../abc   |
      | abc/..   |
      | abc/../d |
      | abc//    |
      | abc//d   |
    Then `InvalidArgument` error is returned
