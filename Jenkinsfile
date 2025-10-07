// Jenkins Pipeline for MockForge Integration Testing

pipeline {
    agent any

    environment {
        DOCKER_IMAGE = "mockforge:${env.BUILD_ID}"
        DOCKER_REGISTRY = credentials('docker-registry')
    }

    stages {
        stage('Build') {
            steps {
                script {
                    echo 'Building MockForge Docker image...'
                    sh 'docker build -t ${DOCKER_IMAGE} .'
                }
            }
        }

        stage('Validate Contracts') {
            when {
                anyOf {
                    branch 'main'
                    changeRequest()
                }
            }
            steps {
                script {
                    echo 'Validating API contracts...'

                    // Install MockForge CLI
                    sh 'cargo install --path crates/mockforge-cli'

                    // Validate contracts
                    sh '''
                        mockforge-cli validate \
                            --spec specs/api.yaml \
                            --endpoint ${API_URL} \
                            --strict \
                            --output validation-report.md
                    '''
                }
            }
            post {
                always {
                    archiveArtifacts artifacts: 'validation-report.md', allowEmptyArchive: true
                }
            }
        }

        stage('Detect Breaking Changes') {
            when {
                changeRequest()
            }
            steps {
                script {
                    echo 'Checking for breaking changes...'

                    sh '''
                        # Get main branch spec
                        git fetch origin main
                        git show origin/main:specs/api.yaml > old-spec.yaml

                        # Compare specs
                        mockforge-cli compare \
                            --old old-spec.yaml \
                            --new specs/api.yaml \
                            --output breaking-changes.md
                    '''

                    // Check if breaking changes exist
                    def hasBreakingChanges = sh(
                        script: 'grep -q "Breaking Changes" breaking-changes.md',
                        returnStatus: true
                    ) == 0

                    if (hasBreakingChanges) {
                        echo 'WARNING: Breaking changes detected!'
                        currentBuild.result = 'UNSTABLE'

                        // Post comment to PR if using GitHub/GitLab
                        def report = readFile('breaking-changes.md')
                        echo "Breaking Changes Report:\n${report}"
                    }
                }
            }
            post {
                always {
                    archiveArtifacts artifacts: 'breaking-changes.md', allowEmptyArchive: true
                }
            }
        }

        stage('Start Mock Services') {
            steps {
                script {
                    echo 'Starting MockForge services...'

                    // Start docker-compose services
                    sh 'docker-compose -f docker-compose.microservices.yml up -d'

                    // Wait for services to be healthy
                    sh '''
                        for port in 3001 3002 3003 3004; do
                            timeout 60 bash -c "until curl -f http://localhost:$port/health; do sleep 2; done"
                            echo "✓ Service on port $port is ready"
                        done
                    '''
                }
            }
        }

        stage('Integration Tests') {
            steps {
                script {
                    echo 'Running integration tests...'

                    sh '''
                        npm install
                        npm test
                    '''
                }
            }
            post {
                always {
                    // Publish test results
                    junit 'test-results/**/*.xml'

                    // Publish coverage reports
                    publishHTML([
                        allowMissing: false,
                        alwaysLinkToLastBuild: true,
                        keepAll: true,
                        reportDir: 'coverage',
                        reportFiles: 'index.html',
                        reportName: 'Coverage Report'
                    ])

                    // Archive artifacts
                    archiveArtifacts artifacts: 'test-results/**/*', allowEmptyArchive: true
                }
            }
        }

        stage('Stop Mock Services') {
            steps {
                script {
                    echo 'Stopping MockForge services...'

                    // Get logs before stopping
                    sh 'docker-compose -f docker-compose.microservices.yml logs > mockforge-logs.txt'

                    // Stop services
                    sh 'docker-compose -f docker-compose.microservices.yml down'
                }
            }
            post {
                always {
                    archiveArtifacts artifacts: 'mockforge-logs.txt', allowEmptyArchive: true
                }
            }
        }

        stage('Deploy to Staging') {
            when {
                branch 'main'
            }
            steps {
                script {
                    echo 'Deploying to staging...'

                    sh '''
                        docker tag ${DOCKER_IMAGE} ${DOCKER_REGISTRY}/mockforge:staging
                        docker push ${DOCKER_REGISTRY}/mockforge:staging
                    '''

                    // Deploy to Kubernetes or your platform
                    sh 'kubectl set image deployment/mockforge mockforge=${DOCKER_REGISTRY}/mockforge:staging'
                }
            }
        }

        stage('Deploy to Production') {
            when {
                tag pattern: "v\\d+\\.\\d+\\.\\d+", comparator: "REGEXP"
            }
            steps {
                input message: 'Deploy to production?', ok: 'Deploy'

                script {
                    echo 'Deploying to production...'

                    sh '''
                        docker tag ${DOCKER_IMAGE} ${DOCKER_REGISTRY}/mockforge:${TAG_NAME}
                        docker tag ${DOCKER_IMAGE} ${DOCKER_REGISTRY}/mockforge:latest
                        docker push ${DOCKER_REGISTRY}/mockforge:${TAG_NAME}
                        docker push ${DOCKER_REGISTRY}/mockforge:latest
                    '''

                    sh 'kubectl set image deployment/mockforge mockforge=${DOCKER_REGISTRY}/mockforge:${TAG_NAME}'
                }
            }
        }
    }

    post {
        always {
            echo 'Cleaning up...'
            sh 'docker system prune -f'
        }
        success {
            echo 'Pipeline completed successfully!'
        }
        failure {
            echo 'Pipeline failed!'
            // Send notifications (email, Slack, etc.)
        }
    }
}
