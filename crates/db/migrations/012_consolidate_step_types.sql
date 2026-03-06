-- Consolidate workflow step types: Plan/Implement/PrReview/CompletePr → agentic, HumanReview → human_gate

UPDATE workflow_step_outputs SET step_type = 'agentic'
    WHERE step_type IN ('plan', 'implement', 'pr_review', 'complete_pr');

UPDATE workflow_step_outputs SET step_type = 'human_gate'
    WHERE step_type = 'human_review';
