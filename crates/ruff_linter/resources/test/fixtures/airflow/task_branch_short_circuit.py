from airflow.decorators import task

@task.branch
def my_task():
    if condition1:
        return []
    if condition2:
        return ["my_downstream_task"]
    return []

my_task >> my_downstream_task

@task.branch
def another_task():
    if condition1:
        return []
    if condition2:
        return ["another_downstream_task"]
    if condition3:
        return []
    return []

another_task >> another_downstream_task

@task.branch
def yet_another_task():
    if condition1:
        return ["yet_another_downstream_task"]
    if condition2:
        return ["yet_another_downstream_task"]
    return []

yet_another_task >> yet_another_downstream_task

@task.branch
def no_short_circuit_task():
    if condition1:
        return []
    if condition2:
        return []
    return []

no_short_circuit_task >> no_short_circuit_downstream_task
