name: Feature Request
about: Suggest an idea for Refractium
title: "[FEAT] <title>"
labels: enhancement
assignees: ''

body:
  - type: markdown
    attributes:
      value: |
        Thank you for suggesting a new feature! We appreciate your input.
  - type: textarea
    id: summary
    attributes:
      label: Summary
      placeholder: Describe the feature you'd like to see.
    validations:
      required: true
  - type: textarea
    id: problem
    attributes:
      label: Problem Statement
      placeholder: Is your feature request related to a problem? Please describe.
    validations:
      required: true
  - type: textarea
    id: solution
    attributes:
      label: Proposed Solution
      placeholder: Describe the solution you'd like.
    validations:
      required: true
  - type: textarea
    id: alternatives
    attributes:
      label: Alternatives Considered
      placeholder: Describe any alternative solutions or features you've considered.
    validations:
      required: false
  - type: textarea
    id: context
    attributes:
      label: Additional Context
      placeholder: Add any other context or screenshots about the feature request here.
    validations:
      required: false
