CREATE TABLE users (
  id UUID PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  role TEXT NOT NULL,
  supplier_id UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE suppliers (
  id UUID PRIMARY KEY,
  cnpj TEXT NOT NULL UNIQUE,
  legal_name TEXT NOT NULL,
  contact_email TEXT NOT NULL,
  phone TEXT,
  status TEXT NOT NULL DEFAULT 'PENDING',
  status_reason TEXT,
  decided_at TIMESTAMPTZ,
  decided_by UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE users
  ADD CONSTRAINT users_supplier_fk FOREIGN KEY (supplier_id) REFERENCES suppliers(id);

CREATE TABLE supplier_documents (
  id UUID PRIMARY KEY,
  supplier_id UUID NOT NULL REFERENCES suppliers(id),
  doc_type TEXT NOT NULL,
  file_name TEXT NOT NULL,
  file_path TEXT NOT NULL,
  valid_until DATE,
  uploaded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX supplier_documents_supplier_idx ON supplier_documents(supplier_id);

CREATE TABLE quotations (
  id UUID PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL DEFAULT 'DRAFT',
  passenger_name TEXT NOT NULL,
  passenger_cpf TEXT NOT NULL,
  passenger_sex TEXT NOT NULL,
  passenger_birth DATE NOT NULL,
  origin TEXT NOT NULL,
  destination TEXT NOT NULL,
  departure_at TIMESTAMPTZ NOT NULL,
  return_at TIMESTAMPTZ,
  reference_flight TEXT NOT NULL,
  reference_price_cents BIGINT NOT NULL,
  opens_at TIMESTAMPTZ,
  closes_at TIMESTAMPTZ,
  awarded_proposal_id UUID UNIQUE,
  awarded_at TIMESTAMPTZ,
  award_justification TEXT,
  ticket_deadline_at TIMESTAMPTZ,
  created_by UUID NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE proposals (
  id UUID PRIMARY KEY,
  quotation_id UUID NOT NULL REFERENCES quotations(id),
  supplier_id UUID NOT NULL REFERENCES suppliers(id),
  total_price_cents BIGINT NOT NULL,
  flight_info TEXT NOT NULL,
  notes TEXT,
  submitted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (quotation_id, supplier_id)
);

CREATE TABLE service_orders (
  id UUID PRIMARY KEY,
  quotation_id UUID NOT NULL UNIQUE REFERENCES quotations(id),
  number TEXT NOT NULL UNIQUE,
  issued_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE tickets (
  id UUID PRIMARY KEY,
  quotation_id UUID NOT NULL UNIQUE REFERENCES quotations(id),
  file_name TEXT NOT NULL,
  file_path TEXT NOT NULL,
  passenger_name TEXT NOT NULL,
  flight_info TEXT NOT NULL,
  departure_at TIMESTAMPTZ NOT NULL,
  price_cents BIGINT NOT NULL,
  divergences JSONB NOT NULL,
  late BOOLEAN NOT NULL,
  uploaded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  confirmed_at TIMESTAMPTZ,
  confirmed_by UUID
);

CREATE TABLE notifications (
  id UUID PRIMARY KEY,
  supplier_id UUID NOT NULL REFERENCES suppliers(id),
  quotation_id UUID REFERENCES quotations(id),
  kind TEXT NOT NULL,
  message TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  read_at TIMESTAMPTZ
);

CREATE TABLE audit_events (
  seq BIGSERIAL PRIMARY KEY,
  at TEXT NOT NULL,
  actor_id UUID,
  actor_role TEXT,
  event_type TEXT NOT NULL,
  entity TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  quotation_id UUID,
  payload JSONB NOT NULL,
  prev_hash TEXT NOT NULL,
  hash TEXT NOT NULL
);
CREATE INDEX audit_events_quotation_idx ON audit_events(quotation_id);

CREATE TABLE counters (
  id TEXT PRIMARY KEY,
  value BIGINT NOT NULL
);
