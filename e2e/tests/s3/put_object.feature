Feature: PutObject

  Scenario: PutObject: regular file
    When 'mydir/myfile' file is uploaded to 'data' bucket
    Then the file is stored as 'data/mydir/myfile'

  Scenario: PutObject: symlink
    When 'myfile' file is uploaded to 'data' bucket
     And 'mydir/mylink' symlink is created on 'links' bucket pointing to 'data/myfile'
    Then the file is accessible via 'links/mydir/mylink'

  Scenario: PubObject: invalid keys: root component
    Given keys with leading '/' are considered invalid

  Scenario: PutObject: invalid keys: current/parent dir and empty components
    Given keys containing '.', '..', '//' path components are considered invalid
