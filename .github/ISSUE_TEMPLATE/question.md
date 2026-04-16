name: Question
about: Ask a question or seek guidance about Refractium
title: "[QUESTION] <title>"
labels: question
assignees: ''

body:
  - type: markdown
    attributes:
      value: |
        Have a question about Refractium? Ask here! Please check the existing documentation first.
  - type: textarea
    id: summary
    attributes:
      label: Summary
      placeholder: What is your question?
    validations:
      required: true
  - type: textarea
    id: context
    attributes:
      label: Context
      placeholder: Provide any context that might help us answer your question.
    validations:
      required: false
