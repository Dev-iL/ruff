from __future__ import annotations

import collections
import collections.abc
import typing
from collections.abc import Mapping
from typing import Dict, MutableMapping

from airflow.decorators import task
from airflow.sdk import task as sdk_task


# === Positive cases ===

# Dict literal return, no annotation -> infers False
@task
def no_annotation_dict():
    return {"x": 1}


# `-> dict` annotation -> infers True
@task
def annotated_dict() -> dict:
    return {"x": 1}


# Subscripted dict annotation -> infers True
@task
def annotated_dict_subscript() -> dict[str, int]:
    return {"x": 1}


# typing.Dict -> infers True
@task
def annotated_typing_dict() -> Dict[str, int]:
    return {"x": 1}


# typing.Mapping -> infers True
@task
def annotated_mapping() -> Mapping[str, int]:
    return {"x": 1}


# typing.MutableMapping -> infers True
@task
def annotated_mutable_mapping() -> MutableMapping[str, int]:
    return {"x": 1}


# collections.abc.Mapping (fully qualified) -> infers True
@task
def annotated_qualified_mapping() -> collections.abc.Mapping[str, int]:
    return {"x": 1}


# collections.OrderedDict -> infers True
@task
def annotated_ordered_dict() -> collections.OrderedDict:
    return {"x": 1}


# Empty call form
@task()
def empty_call_form():
    return {"x": 1}


# Call form with existing kwargs
@task(retries=3)
def call_form_with_kwargs() -> dict:
    return {"x": 1}


# Variant: @task.virtualenv
@task.virtualenv(requirements=["pandas"])
def virtualenv_variant() -> dict:
    return {"x": 1}


# Variant: @task.short_circuit returning dict literal still triggers (user chose "all variants")
@task.short_circuit
def short_circuit_variant():
    return {"go": True}


# Dict comprehension
@task
def dict_comprehension():
    return {k: v for k, v in [("a", 1)]}


# Conditional returns, one of which is a dict
@task
def conditional_dict_return(flag: bool):
    if flag:
        return {"x": 1}
    return None


# Using the airflow.sdk alias
@sdk_task
def via_sdk_alias() -> dict:
    return {"x": 1}


# === Negative cases (should NOT trigger) ===

# Already specifies multiple_outputs
@task(multiple_outputs=True)
def already_explicit_true() -> dict:
    return {"x": 1}


@task(multiple_outputs=False)
def already_explicit_false():
    return {"x": 1}


# Returns a list (not a Mapping)
@task
def returns_list():
    return [1, 2, 3]


# Returns a scalar
@task
def returns_scalar():
    return 42


# Not decorated with @task
def not_a_task():
    return {"x": 1}


# Nested function returns dict, but outer task does not
@task
def nested_dict_inside():
    def inner():
        return {"x": 1}

    return inner


# Annotation that isn't a Mapping
@task
def annotated_list() -> list:
    return [1, 2, 3]
