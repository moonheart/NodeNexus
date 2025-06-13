-- Migration to create batch_command_tasks and child_command_tasks tables

-- Create batch_command_tasks table
CREATE TABLE public.batch_command_tasks
(
    batch_command_id           UUID                                   NOT NULL PRIMARY KEY,
    original_request_payload   JSONB                                  NOT NULL,
    status                     VARCHAR(50)                            NOT NULL, -- PENDING, IN_PROGRESS, COMPLETED_SUCCESSFULLY, COMPLETED_WITH_ERRORS, TERMINATED, FAILED_TO_START
    execution_alias            VARCHAR(255),
    user_id                    INTEGER                                NOT NULL REFERENCES public.users (id) ON DELETE CASCADE,
    created_at                 TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at                 TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    completed_at               TIMESTAMP WITH TIME ZONE
);

ALTER TABLE public.batch_command_tasks
    OWNER TO postgres; -- Adjust owner if necessary

-- Add indexes for batch_command_tasks
CREATE INDEX idx_batch_command_tasks_status ON public.batch_command_tasks (status);
CREATE INDEX idx_batch_command_tasks_user_id ON public.batch_command_tasks (user_id);
CREATE INDEX idx_batch_command_tasks_created_at ON public.batch_command_tasks (created_at DESC);

-- Create child_command_tasks table
CREATE TABLE public.child_command_tasks
(
    child_command_id           UUID                                   NOT NULL PRIMARY KEY,
    batch_command_id           UUID                                   NOT NULL REFERENCES public.batch_command_tasks (batch_command_id) ON DELETE CASCADE,
    vps_id                     INTEGER                                NOT NULL REFERENCES public.vps (id) ON DELETE CASCADE,
    status                     VARCHAR(50)                            NOT NULL, -- PENDING, SENT_TO_AGENT, AGENT_ACCEPTED, EXECUTING, SUCCESS, FAILURE, TERMINATED, AGENT_UNREACHABLE, AGENT_REJECTED
    exit_code                  INTEGER,
    error_message              TEXT,
    stdout_log_path            VARCHAR(1024),
    stderr_log_path            VARCHAR(1024),
    last_output_at             TIMESTAMP WITH TIME ZONE,
    created_at                 TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at                 TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    agent_started_at           TIMESTAMP WITH TIME ZONE,
    agent_completed_at         TIMESTAMP WITH TIME ZONE
);

ALTER TABLE public.child_command_tasks
    OWNER TO postgres; -- Adjust owner if necessary

-- Add indexes for child_command_tasks
CREATE INDEX idx_child_command_tasks_batch_command_id ON public.child_command_tasks (batch_command_id);
CREATE INDEX idx_child_command_tasks_vps_id ON public.child_command_tasks (vps_id);
CREATE INDEX idx_child_command_tasks_status ON public.child_command_tasks (status);
CREATE INDEX idx_child_command_tasks_created_at ON public.child_command_tasks (created_at DESC);

COMMENT ON COLUMN public.batch_command_tasks.status IS 'Possible values: PENDING, IN_PROGRESS, COMPLETED_SUCCESSFULLY, COMPLETED_WITH_ERRORS, TERMINATED, FAILED_TO_START';
COMMENT ON COLUMN public.child_command_tasks.status IS 'Possible values: PENDING, SENT_TO_AGENT, AGENT_ACCEPTED, EXECUTING, SUCCESS, FAILURE, TERMINATED, AGENT_UNREACHABLE, AGENT_REJECTED';