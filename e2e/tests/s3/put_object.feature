Feature: PutObject

  Scenario: PutObject: regular file
    When a file is uploaded to 'data' bucket using 'mydir/myfile' key
    Then the file is stored as 'data/mydir/myfile'

  Scenario: PutObject: symlink
    When a file is uploaded to 'data' bucket using 'myfile' key
     And a symlink is uploaded to 'links' bucket using 'mydir/mylink' key pointing to 'data/myfile'
    Then the file is accessible via 'links/mydir/mylink'

  Scenario: PubObject: invalid keys: root component
    Given keys with leading '/' are considered invalid

  Scenario: PutObject: invalid keys: current/parent dir and empty components
    Given keys containing '.', '..', '//' path components are considered invalid
