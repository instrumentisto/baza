Feature: GetObject

  Scenario: GetObject: existing file
    Given `rms.jpg` was uploaded to `data` bucket as `my_file`
    When trying to load `my_file` from `data` bucket
    Then `rms.jpg` file is returned

  Scenario: GetObject: non-existing file
    Given there was nothing uploaded to `data` bucket as `my_file`
    When trying to load `my_file` from `data` bucket
    Then `NoSuchKey` error is returned
