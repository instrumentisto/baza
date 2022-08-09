Feature: GetObject

  Scenario: GetObject: existing file
    Given `rms.jpg` was uploaded to `data` bucket as `my_file`
    Then GetObject(`data`, `my_file`) returns `rms.jpg`

  Scenario: GetObject: non-existing file
    Given there was nothing uploaded to `data` bucket as `my_file`
    Then GetObject(`data`, `my_file`) returns `NoSuchKey` error