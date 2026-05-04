// AWS Fargate skeleton for ZeroTrust Gatekeeper. Deliberately ships
// without a backend block — wire your own S3 + DynamoDB lock table
// before running `terraform init`.
//
// Topology (mirrors the GCP module under ../gcp/):
//   - One ECS Fargate service per app (gateway, auth-issuer,
//     backend-echo) on a shared cluster.
//   - An ALB in the public subnets fronts the gateway only;
//     auth-issuer and backend-echo are private and reachable only
//     via service discovery.
//   - ElastiCache Redis (single-node) backs the gateway's rate
//     limiter — set GATEWAY_REDIS_URL on the gateway task.
//   - Per-service security groups so the gateway → auth-issuer /
//     backend-echo path is internal-only.
//   - The auth-issuer task role gets read access to a
//     Secrets Manager secret holding the RS256 signing key; nobody
//     else can read it.

provider "aws" {
  region = var.region
}

locals {
  gateway_port      = 8443
  metrics_port      = 9100
  auth_issuer_port  = 8081
  backend_echo_port = 8082
}

// ---------- ECS cluster ----------

resource "aws_ecs_cluster" "this" {
  name = "${var.name_prefix}-cluster"
  tags = var.tags
}

// ---------- Service discovery (private DNS namespace) ----------
//
// Lets the gateway address auth-issuer / backend-echo by stable
// DNS names without an internal ALB.

resource "aws_service_discovery_private_dns_namespace" "internal" {
  name        = "${var.name_prefix}.internal"
  description = "Private DNS for ZeroTrust Gatekeeper services"
  vpc         = var.vpc_id
  tags        = var.tags
}

resource "aws_service_discovery_service" "auth_issuer" {
  name = "auth-issuer"
  dns_config {
    namespace_id = aws_service_discovery_private_dns_namespace.internal.id
    dns_records {
      ttl  = 10
      type = "A"
    }
    routing_policy = "MULTIVALUE"
  }
  health_check_custom_config {
    failure_threshold = 1
  }
  tags = var.tags
}

resource "aws_service_discovery_service" "backend_echo" {
  name = "backend-echo"
  dns_config {
    namespace_id = aws_service_discovery_private_dns_namespace.internal.id
    dns_records {
      ttl  = 10
      type = "A"
    }
    routing_policy = "MULTIVALUE"
  }
  health_check_custom_config {
    failure_threshold = 1
  }
  tags = var.tags
}

// ---------- Security groups ----------

resource "aws_security_group" "alb" {
  name        = "${var.name_prefix}-alb"
  description = "Public ingress to the gateway via the ALB."
  vpc_id      = var.vpc_id

  ingress {
    description = "HTTPS from the internet"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    description = "ALB to gateway tasks"
    from_port   = local.gateway_port
    to_port     = local.gateway_port
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = var.tags
}

resource "aws_security_group" "gateway" {
  name        = "${var.name_prefix}-gateway"
  description = "Gateway tasks: accept from ALB, talk to auth-issuer / backend-echo / Redis."
  vpc_id      = var.vpc_id

  ingress {
    description     = "ALB to gateway"
    from_port       = local.gateway_port
    to_port         = local.gateway_port
    protocol        = "tcp"
    security_groups = [aws_security_group.alb.id]
  }

  egress {
    description = "All egress (auth-issuer, backend-echo, Redis, internet for ECR/CloudWatch)"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = var.tags
}

resource "aws_security_group" "auth_issuer" {
  name        = "${var.name_prefix}-auth-issuer"
  description = "auth-issuer: accept from gateway only."
  vpc_id      = var.vpc_id

  ingress {
    description     = "gateway to auth-issuer"
    from_port       = local.auth_issuer_port
    to_port         = local.auth_issuer_port
    protocol        = "tcp"
    security_groups = [aws_security_group.gateway.id]
  }

  egress {
    description = "Egress for ECR / Secrets Manager / CloudWatch"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = var.tags
}

resource "aws_security_group" "backend_echo" {
  name        = "${var.name_prefix}-backend-echo"
  description = "backend-echo: accept from gateway only."
  vpc_id      = var.vpc_id

  ingress {
    description     = "gateway to backend-echo"
    from_port       = local.backend_echo_port
    to_port         = local.backend_echo_port
    protocol        = "tcp"
    security_groups = [aws_security_group.gateway.id]
  }

  egress {
    description = "Egress for ECR / CloudWatch"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = var.tags
}

resource "aws_security_group" "redis" {
  name        = "${var.name_prefix}-redis"
  description = "ElastiCache: accept from gateway only."
  vpc_id      = var.vpc_id

  ingress {
    description     = "gateway to Redis"
    from_port       = 6379
    to_port         = 6379
    protocol        = "tcp"
    security_groups = [aws_security_group.gateway.id]
  }

  tags = var.tags
}

// ---------- ElastiCache Redis ----------

resource "aws_elasticache_subnet_group" "redis" {
  name       = "${var.name_prefix}-redis"
  subnet_ids = var.private_subnet_ids
  tags       = var.tags
}

resource "aws_elasticache_replication_group" "rate_limit" {
  replication_group_id = "${var.name_prefix}-rate-limit"
  description          = "ZeroTrust gateway rate limit"
  node_type            = "cache.t4g.micro"
  num_cache_clusters   = 1
  engine               = "redis"
  engine_version       = "7.1"
  port                 = 6379
  parameter_group_name = "default.redis7"
  subnet_group_name    = aws_elasticache_subnet_group.redis.name
  security_group_ids   = [aws_security_group.redis.id]
  apply_immediately    = true
  tags                 = var.tags
}

// ---------- IAM ----------

data "aws_iam_policy_document" "task_assume" {
  statement {
    actions = ["sts:AssumeRole"]
    principals {
      type        = "Service"
      identifiers = ["ecs-tasks.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "task_execution" {
  name               = "${var.name_prefix}-task-execution"
  assume_role_policy = data.aws_iam_policy_document.task_assume.json
  tags               = var.tags
}

resource "aws_iam_role_policy_attachment" "task_execution" {
  role       = aws_iam_role.task_execution.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

resource "aws_iam_role" "auth_issuer_task" {
  name               = "${var.name_prefix}-auth-issuer-task"
  assume_role_policy = data.aws_iam_policy_document.task_assume.json
  tags               = var.tags
}

data "aws_iam_policy_document" "auth_issuer_secret_read" {
  statement {
    actions   = ["secretsmanager:GetSecretValue"]
    resources = [var.auth_signing_secret_arn]
  }
}

resource "aws_iam_role_policy" "auth_issuer_secret_read" {
  name   = "${var.name_prefix}-auth-issuer-secret-read"
  role   = aws_iam_role.auth_issuer_task.id
  policy = data.aws_iam_policy_document.auth_issuer_secret_read.json
}

resource "aws_iam_role" "gateway_task" {
  name               = "${var.name_prefix}-gateway-task"
  assume_role_policy = data.aws_iam_policy_document.task_assume.json
  tags               = var.tags
}

resource "aws_iam_role" "backend_echo_task" {
  name               = "${var.name_prefix}-backend-echo-task"
  assume_role_policy = data.aws_iam_policy_document.task_assume.json
  tags               = var.tags
}

// ---------- CloudWatch log groups ----------

resource "aws_cloudwatch_log_group" "gateway" {
  name              = "/ecs/${var.name_prefix}/gateway"
  retention_in_days = 30
  tags              = var.tags
}

resource "aws_cloudwatch_log_group" "auth_issuer" {
  name              = "/ecs/${var.name_prefix}/auth-issuer"
  retention_in_days = 30
  tags              = var.tags
}

resource "aws_cloudwatch_log_group" "backend_echo" {
  name              = "/ecs/${var.name_prefix}/backend-echo"
  retention_in_days = 30
  tags              = var.tags
}

// ---------- Task definitions ----------

resource "aws_ecs_task_definition" "auth_issuer" {
  family                   = "${var.name_prefix}-auth-issuer"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = aws_iam_role.task_execution.arn
  task_role_arn            = aws_iam_role.auth_issuer_task.arn

  container_definitions = jsonencode([{
    name      = "auth-issuer"
    image     = var.auth_issuer_image
    essential = true
    portMappings = [{
      containerPort = local.auth_issuer_port
      protocol      = "tcp"
    }]
    environment = [
      { name = "AUTH_ADDR", value = ":${local.auth_issuer_port}" },
      { name = "AUTH_TOKEN_TTL", value = "15m" },
    ]
    secrets = [
      { name = "AUTH_SIGNING_KEY", valueFrom = var.auth_signing_secret_arn },
    ]
    logConfiguration = {
      logDriver = "awslogs"
      options = {
        awslogs-group         = aws_cloudwatch_log_group.auth_issuer.name
        awslogs-region        = var.region
        awslogs-stream-prefix = "ecs"
      }
    }
  }])

  tags = var.tags
}

resource "aws_ecs_task_definition" "backend_echo" {
  family                   = "${var.name_prefix}-backend-echo"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "256"
  memory                   = "512"
  execution_role_arn       = aws_iam_role.task_execution.arn
  task_role_arn            = aws_iam_role.backend_echo_task.arn

  container_definitions = jsonencode([{
    name      = "backend-echo"
    image     = var.backend_echo_image
    essential = true
    portMappings = [{
      containerPort = local.backend_echo_port
      protocol      = "tcp"
    }]
    environment = [
      { name = "ECHO_ADDR", value = ":${local.backend_echo_port}" },
    ]
    logConfiguration = {
      logDriver = "awslogs"
      options = {
        awslogs-group         = aws_cloudwatch_log_group.backend_echo.name
        awslogs-region        = var.region
        awslogs-stream-prefix = "ecs"
      }
    }
  }])

  tags = var.tags
}

resource "aws_ecs_task_definition" "gateway" {
  family                   = "${var.name_prefix}-gateway"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = "512"
  memory                   = "1024"
  execution_role_arn       = aws_iam_role.task_execution.arn
  task_role_arn            = aws_iam_role.gateway_task.arn

  container_definitions = jsonencode([{
    name      = "gateway"
    image     = var.gateway_image
    essential = true
    portMappings = [
      { containerPort = local.gateway_port, protocol = "tcp" },
      { containerPort = local.metrics_port, protocol = "tcp" },
    ]
    environment = [
      { name = "GATEWAY_ADDR", value = "0.0.0.0:${local.gateway_port}" },
      { name = "GATEWAY_METRICS_ADDR", value = "0.0.0.0:${local.metrics_port}" },
      { name = "GATEWAY_UPSTREAM_URL", value = "http://backend-echo.${aws_service_discovery_private_dns_namespace.internal.name}:${local.backend_echo_port}" },
      { name = "GATEWAY_AUTH_JWKS_URL", value = "http://auth-issuer.${aws_service_discovery_private_dns_namespace.internal.name}:${local.auth_issuer_port}/.well-known/jwks.json" },
      { name = "GATEWAY_AUTH_ISSUER", value = "zt-auth-issuer" },
      { name = "GATEWAY_AUTH_AUDIENCE", value = "zt-gateway" },
      { name = "GATEWAY_REDIS_URL", value = "redis://${aws_elasticache_replication_group.rate_limit.primary_endpoint_address}:6379" },
      { name = "RUST_LOG", value = "info" },
    ]
    logConfiguration = {
      logDriver = "awslogs"
      options = {
        awslogs-group         = aws_cloudwatch_log_group.gateway.name
        awslogs-region        = var.region
        awslogs-stream-prefix = "ecs"
      }
    }
  }])

  tags = var.tags
}

// ---------- ALB ----------

resource "aws_lb" "gateway" {
  name               = "${var.name_prefix}-gateway"
  load_balancer_type = "application"
  internal           = false
  subnets            = var.public_subnet_ids
  security_groups    = [aws_security_group.alb.id]
  tags               = var.tags
}

resource "aws_lb_target_group" "gateway" {
  name        = "${var.name_prefix}-gateway-tg"
  port        = local.gateway_port
  protocol    = "HTTPS"
  target_type = "ip"
  vpc_id      = var.vpc_id

  health_check {
    path                = "/healthz"
    protocol            = "HTTPS"
    matcher             = "200"
    interval            = 30
    healthy_threshold   = 2
    unhealthy_threshold = 5
  }

  tags = var.tags
}

resource "aws_lb_listener" "https" {
  load_balancer_arn = aws_lb.gateway.arn
  port              = 443
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.gateway_certificate_arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.gateway.arn
  }

  tags = var.tags
}

// ---------- ECS services ----------

resource "aws_ecs_service" "auth_issuer" {
  name             = "${var.name_prefix}-auth-issuer"
  cluster          = aws_ecs_cluster.this.id
  task_definition  = aws_ecs_task_definition.auth_issuer.arn
  desired_count    = 1
  launch_type      = "FARGATE"
  platform_version = "LATEST"

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [aws_security_group.auth_issuer.id]
    assign_public_ip = false
  }

  service_registries {
    registry_arn = aws_service_discovery_service.auth_issuer.arn
  }

  tags = var.tags
}

resource "aws_ecs_service" "backend_echo" {
  name             = "${var.name_prefix}-backend-echo"
  cluster          = aws_ecs_cluster.this.id
  task_definition  = aws_ecs_task_definition.backend_echo.arn
  desired_count    = 1
  launch_type      = "FARGATE"
  platform_version = "LATEST"

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [aws_security_group.backend_echo.id]
    assign_public_ip = false
  }

  service_registries {
    registry_arn = aws_service_discovery_service.backend_echo.arn
  }

  tags = var.tags
}

resource "aws_ecs_service" "gateway" {
  name             = "${var.name_prefix}-gateway"
  cluster          = aws_ecs_cluster.this.id
  task_definition  = aws_ecs_task_definition.gateway.arn
  desired_count    = 2
  launch_type      = "FARGATE"
  platform_version = "LATEST"

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [aws_security_group.gateway.id]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.gateway.arn
    container_name   = "gateway"
    container_port   = local.gateway_port
  }

  depends_on = [aws_lb_listener.https]
  tags       = var.tags
}
