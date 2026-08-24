FROM golang:1.27-alpine AS build
WORKDIR /src
COPY go.mod go.sum ./
COPY . ./
RUN CGO_ENABLED=0 go build -o /out/backend-echo .

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /out/backend-echo /usr/local/bin/backend-echo
EXPOSE 8082
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/backend-echo"]
