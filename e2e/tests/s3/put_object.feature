Feature: PutObject

  Scenario: PutObject: regular file
    When 'mydir/myfile' file is uploaded to 'data' bucket
    Then the file is stored as 'data/mydir/myfile'

  Scenario: PutObject: symlink
    When 'myfile' file is uploaded to 'data' bucket
     And 'mydir/mylink' symlink is created on 'links' bucket pointing to 'data/myfile'
    Then the file is accessible via 'links/mydir/mylink'

  Scenario: PubObject: invalid keys: root component
    When trying to upload files with the following keys:
    | /abc   |
    | /abc/d |
    Then 'InvalidArgument' error is returned

  Scenario: PutObject: invalid keys: current/parent dir and empty components
    When trying to upload files with the following keys:
    | ./abc    |
    | abc/.    |
    | abc/./d  |
    | ../abc   |
    | abc/..   |
    | abc/../d |
    | abc//    |
    | abc//d   |
    Then 'InvalidArgument' error is returned
