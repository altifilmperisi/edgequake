resource "google_service_account" "gce" {
  account_id   = "${var.name_prefix}-gce"
  display_name = "EdgeQuake runtime on elitizon-db"
  project      = var.project_id

  depends_on = [google_project_service.required]
}

resource "google_service_account" "github" {
  account_id   = "${var.name_prefix}-github"
  display_name = "EdgeQuake GitHub Actions deploy (WIF)"
  project      = var.project_id

  depends_on = [google_project_service.required]
}

resource "google_project_iam_member" "gce_logging" {
  project = var.project_id
  role    = "roles/logging.logWriter"
  member  = "serviceAccount:${google_service_account.gce.email}"
}

resource "google_project_iam_member" "gce_monitoring" {
  project = var.project_id
  role    = "roles/monitoring.metricWriter"
  member  = "serviceAccount:${google_service_account.gce.email}"
}

resource "google_storage_bucket_iam_member" "gce_artifacts" {
  bucket = google_storage_bucket.artifacts.name
  role   = "roles/storage.objectViewer"
  member = "serviceAccount:${google_service_account.gce.email}"
}

resource "google_iap_tunnel_instance_iam_member" "github_iap" {
  project  = var.project_id
  zone     = var.zone
  instance = google_compute_instance.gce.name
  role     = "roles/iap.tunnelResourceAccessor"
  member   = "serviceAccount:${google_service_account.github.email}"

  condition {
    title       = "ssh-port-22"
    description = "IAP TCP forwarding only to SSH on elitizon-db"
    expression  = "destination.port == 22"
  }
}

resource "google_compute_instance_iam_member" "github_oslogin" {
  project       = var.project_id
  zone          = var.zone
  instance_name = google_compute_instance.gce.name
  role          = "roles/compute.osAdminLogin"
  member        = "serviceAccount:${google_service_account.github.email}"
}

resource "google_service_account_iam_member" "github_use_gce" {
  service_account_id = google_service_account.gce.name
  role               = "roles/iam.serviceAccountUser"
  member             = "serviceAccount:${google_service_account.github.email}"
}

resource "google_compute_instance_iam_member" "github_viewer" {
  project       = var.project_id
  zone          = var.zone
  instance_name = google_compute_instance.gce.name
  role          = "roles/compute.viewer"
  member        = "serviceAccount:${google_service_account.github.email}"
}

# gcloud compute ssh resolves the project before the instance. Instance-scoped
# compute.viewer does not include compute.projects.get — without this binding
# CD fails with: Required 'compute.projects.get' permission for 'projects/…'
resource "google_project_iam_member" "github_compute_viewer" {
  project = var.project_id
  role    = "roles/compute.viewer"
  member  = "serviceAccount:${google_service_account.github.email}"
}

resource "google_project_iam_member" "operators_iap" {
  for_each = toset(var.operator_members)
  project  = var.project_id
  role     = "roles/iap.tunnelResourceAccessor"
  member   = each.value
}

resource "google_project_iam_member" "operators_oslogin" {
  for_each = toset(var.operator_members)
  project  = var.project_id
  role     = "roles/compute.osAdminLogin"
  member   = each.value
}

resource "google_service_account_iam_member" "operators_use_gce" {
  for_each           = toset(var.operator_members)
  service_account_id = google_service_account.gce.name
  role               = "roles/iam.serviceAccountUser"
  member             = each.value
}
