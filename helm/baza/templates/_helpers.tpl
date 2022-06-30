{{/*
Expand the name of the chart.
*/}}
{{- define "baza.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 55 | trimSuffix "-" -}}
{{- end -}}

{{/*
Create a default fully qualified app name.
We truncate at 55 chars because some Kubernetes name fields are limited to this
(by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "baza.fullname" -}}
{{- if .Values.fullnameOverride -}}
  {{- .Values.fullnameOverride | trunc 55 | trimSuffix "-" -}}
{{- else -}}
  {{- $name := default .Chart.Name .Values.nameOverride -}}
  {{- if contains $name .Release.Name -}}
    {{- .Release.Name | trunc 55 | trimSuffix "-" -}}
  {{- else -}}
    {{- printf "%s-%s" .Release.Name $name | trunc 55 | trimSuffix "-" -}}
  {{- end -}}
{{- end -}}
{{- end -}}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "baza.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 55 | trimSuffix "-" -}}
{{- end -}}

{{/*
Call template of the nested sub-chart in its context.
*/}}
{{- define "baza.call-nested" -}}
{{- $dot := index . 0 -}}
{{- $subchart := index . 1 | splitList "." -}}
{{- $template := index . 2 -}}
{{- $values := $dot.Values -}}
{{- range $subchart -}}
  {{- $values = index $values . -}}
{{- end -}}
{{- include $template (dict "Chart" (dict "Name" (last $subchart)) "Values" $values "Release" $dot.Release "Capabilities" $dot.Capabilities) -}}
{{- end -}}
